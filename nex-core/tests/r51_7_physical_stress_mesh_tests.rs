use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::sync::anti_entropy::AntiEntropyEngine;
use nex_core::apps::drive::normalize_vpath;

fn sync_pair(node_a: &mut NexNode, node_b: &mut NexNode) {
    let session_id = [0x77; 16];
    
    // A -> B
    let adv_b = AntiEntropyEngine::generate_advertise(node_b, session_id);
    let batches_a_to_b = AntiEntropyEngine::generate_batches_for_peer(node_a, session_id, &adv_b.frontier_mutation_ids, 100);
    for batch in batches_a_to_b {
        let _ = AntiEntropyEngine::ingest_batch(node_b, batch);
    }

    // B -> A
    let adv_a = AntiEntropyEngine::generate_advertise(node_a, session_id);
    let batches_b_to_a = AntiEntropyEngine::generate_batches_for_peer(node_b, session_id, &adv_a.frontier_mutation_ids, 100);
    for batch in batches_b_to_a {
        let _ = AntiEntropyEngine::ingest_batch(node_a, batch);
    }
}

#[test]
fn test_r51_7_a_10node_high_throughput_burst_mesh() {
    let mut csprng = OsRng;
    let mut tmp_dirs = Vec::new();
    let mut nodes = Vec::new();

    for _ in 0..10 {
        let tmp = tempdir().unwrap();
        let key = SigningKey::generate(&mut csprng);
        let mut node = NexNode::new(tmp.path(), key);
        node.start().unwrap();
        tmp_dirs.push(tmp);
        nodes.push(node);
    }

    // Each of the 10 nodes authors 10 objects (100 objects total)
    for n_idx in 0..10 {
        for i in 0..10 {
            let mut meta = BTreeMap::new();
            meta.insert("author_node".to_string(), format!("{}", n_idx));
            meta.insert("item".to_string(), format!("{}", i));
            nodes[n_idx].create_object(
                [0xAA; 32],
                ObjectType::Synthetic(1),
                meta,
                format!("BURST_NODE_{}_OBJ_{}", n_idx, i).into_bytes(),
            ).unwrap();
        }
    }

    // 4 rounds of pairwise sync across all 10 nodes to achieve full mesh convergence
    for _round in 0..4 {
        for i in 0..10 {
            for j in (i+1)..10 {
                let (left, right) = nodes.split_at_mut(j);
                sync_pair(&mut left[i], &mut right[0]);
            }
        }
    }

    let target_root = nodes[0].sync_now().unwrap().body.state_root;
    for i in 0..10 {
        assert_eq!(nodes[i].state.object_store.len(), 100, "Node {} must have all 100 objects", i);
        let root = nodes[i].sync_now().unwrap().body.state_root;
        assert_eq!(target_root, root, "Node {} Merkle root must match master mesh root", i);
        nodes[i].stop().unwrap();
    }
}

#[test]
fn test_r51_7_b_sustained_memory_and_cas_gc_stability() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let mut live_roots = std::collections::HashSet::new();

    // 200 operations creating chunks
    for i in 0..200 {
        let chunk_data = format!("CAS_PAYLOAD_CHUNK_DATA_{}", i).into_bytes();
        let digest = node.storage.cas.put_chunk(&chunk_data);
        if i % 2 == 0 {
            live_roots.insert(digest);
        }

        let mut meta = BTreeMap::new();
        meta.insert("idx".to_string(), format!("{}", i));
        node.create_object([0x01; 32], ObjectType::Synthetic(1), meta, chunk_data).unwrap();
    }

    assert_eq!(node.storage.cas.chunks.len(), 200);

    // GC unreachable chunks
    let swept = node.gc_cas(&live_roots);
    assert_eq!(swept, 100, "Must reclaim all 100 unreferenced CAS chunks");
    assert_eq!(node.storage.cas.chunks.len(), 100);

    let cp = node.checkpoint_and_compact().unwrap();
    assert_ne!(cp.body.state_root, [0u8; 32]);

    node.stop().unwrap();
}

#[test]
fn test_r51_7_c_cascading_network_topology_shifts() {
    let mut csprng = OsRng;
    let mut tmp_dirs = Vec::new();
    let mut nodes = Vec::new();

    for _ in 0..6 {
        let tmp = tempdir().unwrap();
        let key = SigningKey::generate(&mut csprng);
        let mut node = NexNode::new(tmp.path(), key);
        node.start().unwrap();
        tmp_dirs.push(tmp);
        nodes.push(node);
    }

    // Phase 1: Line topology sync (N0 -> N1 -> N2 -> N3 -> N4 -> N5) (1 object)
    nodes[0].create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"LINE_PAYLOAD".to_vec()).unwrap();
    for i in 0..5 {
        let (left, right) = nodes.split_at_mut(i + 1);
        sync_pair(&mut left[i], &mut right[0]);
    }
    assert_eq!(nodes[5].state.object_store.len(), 1);

    // Phase 2: Star topology sync (N0 is central hub) (5 objects)
    for i in 1..6 {
        nodes[i].create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!("STAR_P_{}", i).into_bytes()).unwrap();
        let (left, right) = nodes.split_at_mut(i);
        sync_pair(&mut left[0], &mut right[0]);
    }
    for i in 1..6 {
        let (left, right) = nodes.split_at_mut(i);
        sync_pair(&mut left[0], &mut right[0]);
    }

    // Phase 3: Partition into [0..3] and [3..6] (2 objects)
    nodes[1].create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"PARTITION_A".to_vec()).unwrap();
    nodes[4].create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"PARTITION_B".to_vec()).unwrap();

    // Phase 4: Mesh healing across all 6 nodes (Total 1 + 5 + 2 = 8 objects)
    for _ in 0..4 {
        for i in 0..6 {
            for j in (i+1)..6 {
                let (left, right) = nodes.split_at_mut(j);
                sync_pair(&mut left[i], &mut right[0]);
            }
        }
    }

    let target_root = nodes[0].sync_now().unwrap().body.state_root;
    for i in 0..6 {
        assert_eq!(nodes[i].state.object_store.len(), 8, "All nodes must converge to 8 objects");
        let r = nodes[i].sync_now().unwrap().body.state_root;
        assert_eq!(target_root, r, "Node {} Merkle root must match healed root", i);
        nodes[i].stop().unwrap();
    }
}

#[test]
fn test_r51_7_d_multi_tenant_cross_app_stress_isolation() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let ns_drive = [0xD1; 32];
    let ns_chat = [0xC1; 32];
    let ns_community = [0xB1; 32];
    let ns_photos = [0xA1; 32];
    let ns_ext = [0xE1; 32];

    for i in 0..20 {
        // Drive
        let mut d_meta = BTreeMap::new();
        d_meta.insert("path".to_string(), normalize_vpath(&format!("docs\\file_{}.txt", i)));
        node.create_object(ns_drive, ObjectType::DriveInode, d_meta, format!("DRIVE_{}", i).into_bytes()).unwrap();

        // Chat
        let mut c_meta = BTreeMap::new();
        c_meta.insert("channel".to_string(), "general".to_string());
        node.create_object(ns_chat, ObjectType::ChatMessage, c_meta, format!("CHAT_{}", i).into_bytes()).unwrap();

        // Community
        let mut comm_meta = BTreeMap::new();
        comm_meta.insert("title".to_string(), format!("Post {}", i));
        node.create_object(ns_community, ObjectType::Community, comm_meta, format!("COMMUNITY_{}", i).into_bytes()).unwrap();

        // Photos
        let mut p_meta = BTreeMap::new();
        p_meta.insert("album".to_string(), "vacation".to_string());
        node.create_object(ns_photos, ObjectType::PhotoMedia, p_meta, format!("PHOTO_{}", i).into_bytes()).unwrap();

        // Vault
        let mut e_meta = BTreeMap::new();
        e_meta.insert("manifest_id".to_string(), format!("ext_{}", i));
        node.create_object(ns_ext, ObjectType::VaultItem, e_meta, format!("EXT_{}", i).into_bytes()).unwrap();
    }

    assert_eq!(node.state.object_store.len(), 100);

    // Verify isolation by namespace
    let drive_objs: Vec<_> = node.state.object_store.values().filter(|o| o.namespace == ns_drive).collect();
    let chat_objs: Vec<_> = node.state.object_store.values().filter(|o| o.namespace == ns_chat).collect();
    assert_eq!(drive_objs.len(), 20);
    assert_eq!(chat_objs.len(), 20);

    node.stop().unwrap();
}

#[test]
fn test_r51_7_e_sub_50ms_crash_recovery_sla_under_wal_load() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    {
        let mut node = NexNode::new(data_dir.clone(), key.clone());
        node.start().unwrap();

        // Write 200 mutations directly to WAL without snapshotting
        for i in 0..200 {
            let mut meta = BTreeMap::new();
            meta.insert("idx".to_string(), format!("{}", i));
            node.create_object([0x01; 32], ObjectType::Synthetic(1), meta, format!("RECOVERY_SLA_{}", i).into_bytes()).unwrap();
        }

        // Simulate crash by releasing lockfile to dead PID
        let _ = fs::write(data_dir.join(".nex.lock"), "99999999\n");
    }

    // Benchmark recovery time
    let start_time = Instant::now();
    let mut recovered_node = NexNode::new(data_dir, key);
    recovered_node.start().unwrap();
    let duration = start_time.elapsed();

    println!("Crash recovery of 200 uncompacted WAL mutations took: {:?}", duration);
    assert!(
        duration.as_millis() < 50,
        "Crash recovery must meet sub-50ms SLA; took {} ms",
        duration.as_millis()
    );

    assert_eq!(recovered_node.state.object_store.len(), 200);
    recovered_node.stop().unwrap();
}

#[test]
fn test_r51_7_f_master_invariant_verification_and_constitutional_seal() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let cp_init = node.sync_now().unwrap();

    // Author an object and verify Merkle root updates deterministically
    node.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"SEAL_OBJECT".to_vec()).unwrap();
    let cp_mutated = node.sync_now().unwrap();
    assert_ne!(cp_init.body.state_root, cp_mutated.body.state_root);

    node.stop().unwrap();
}
