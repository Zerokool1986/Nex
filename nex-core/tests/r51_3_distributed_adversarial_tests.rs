use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::Rng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::sync::anti_entropy::AntiEntropyEngine;

fn sync_pair(node_a: &mut NexNode, node_b: &mut NexNode) {
    let session_id = [0xAA; 16];
    
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
fn test_r51_3_a_linear_multihop_chain_convergence() {
    let mut csprng = OsRng;
    let mut tmp_dirs = Vec::new();
    let mut nodes = Vec::new();

    // Setup 5 nodes in a line: N0 <-> N1 <-> N2 <-> N3 <-> N4
    for _ in 0..5 {
        let tmp = tempdir().unwrap();
        let key = SigningKey::generate(&mut csprng);
        let mut node = NexNode::new(tmp.path(), key);
        node.start().unwrap();
        tmp_dirs.push(tmp);
        nodes.push(node);
    }

    // N0 authors 5 objects
    for i in 0..5 {
        let mut meta = BTreeMap::new();
        meta.insert("origin".to_string(), "node_0".to_string());
        nodes[0].create_object([0x01; 32], ObjectType::Synthetic(1), meta, format!("N0_OBJ_{}", i).into_bytes()).unwrap();
    }

    // N4 authors 5 objects
    for i in 0..5 {
        let mut meta = BTreeMap::new();
        meta.insert("origin".to_string(), "node_4".to_string());
        nodes[4].create_object([0x02; 32], ObjectType::Synthetic(1), meta, format!("N4_OBJ_{}", i).into_bytes()).unwrap();
    }

    // Propagate multi-hop across adjacent links: 0<->1, 1<->2, 2<->3, 3<->4
    for _ in 0..4 {
        for i in 0..4 {
            let (left, right) = nodes.split_at_mut(i + 1);
            sync_pair(&mut left[i], &mut right[0]);
        }
        for i in (0..4).rev() {
            let (left, right) = nodes.split_at_mut(i + 1);
            sync_pair(&mut left[i], &mut right[0]);
        }
    }

    // Assert all 5 nodes have 10 objects and identical Merkle state commitments
    let root_0 = nodes[0].sync_now().unwrap().body.state_root;
    for i in 0..5 {
        assert_eq!(nodes[i].state.object_store.len(), 10, "Node {} must have all 10 objects", i);
        let root_i = nodes[i].sync_now().unwrap().body.state_root;
        assert_eq!(root_0, root_i, "Node {} Merkle root must match Node 0", i);
        nodes[i].stop().unwrap();
    }
}

#[test]
fn test_r51_3_b_hub_and_spoke_star_topology_convergence() {
    let mut csprng = OsRng;
    let hub_tmp = tempdir().unwrap();
    let hub_key = SigningKey::generate(&mut csprng);
    let mut hub = NexNode::new(hub_tmp.path(), hub_key);
    hub.start().unwrap();

    let mut spokes = Vec::new();
    let mut spoke_tmps = Vec::new();

    for i in 0..4 {
        let tmp = tempdir().unwrap();
        let key = SigningKey::generate(&mut csprng);
        let mut spoke = NexNode::new(tmp.path(), key);
        spoke.start().unwrap();

        // Each spoke authors 5 unique objects
        for j in 0..5 {
            let mut meta = BTreeMap::new();
            meta.insert("spoke".to_string(), format!("{}", i));
            spoke.create_object([0x10 + i as u8; 32], ObjectType::Synthetic(1), meta, format!("SPOKE_{}_OBJ_{}", i, j).into_bytes()).unwrap();
        }

        spoke_tmps.push(tmp);
        spokes.push(spoke);
    }

    // Round 1: Each spoke syncs with Hub (Hub collects all 20 objects)
    for spoke in &mut spokes {
        sync_pair(&mut hub, spoke);
    }
    assert_eq!(hub.state.object_store.len(), 20, "Hub must receive all 20 objects");

    // Round 2: Hub syncs back with all spokes (Distributes all 20 objects to all spokes)
    for spoke in &mut spokes {
        sync_pair(&mut hub, spoke);
    }

    let hub_root = hub.sync_now().unwrap().body.state_root;
    for (i, spoke) in spokes.iter_mut().enumerate() {
        assert_eq!(spoke.state.object_store.len(), 20, "Spoke {} must have all 20 objects", i);
        let spoke_root = spoke.sync_now().unwrap().body.state_root;
        assert_eq!(hub_root, spoke_root, "Spoke {} root must match Hub root", i);
        spoke.stop().unwrap();
    }
    hub.stop().unwrap();
}

#[test]
fn test_r51_3_c_network_partition_healing_and_merkle_invariance() {
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

    // Partition 1 (Nodes 0, 1, 2) authors 10 objects
    for i in 0..10 {
        let mut meta = BTreeMap::new();
        meta.insert("partition".to_string(), "part1".to_string());
        nodes[i % 3].create_object([0xAA; 32], ObjectType::Synthetic(1), meta, format!("P1_OBJ_{}", i).into_bytes()).unwrap();
    }
    // Partition 1 syncs internally
    for i in 0..3 {
        for j in (i+1)..3 {
            let (left, right) = nodes.split_at_mut(j);
            sync_pair(&mut left[i], &mut right[0]);
        }
    }

    // Partition 2 (Nodes 3, 4, 5) authors 10 objects
    for i in 0..10 {
        let mut meta = BTreeMap::new();
        meta.insert("partition".to_string(), "part2".to_string());
        nodes[3 + (i % 3)].create_object([0xBB; 32], ObjectType::Synthetic(1), meta, format!("P2_OBJ_{}", i).into_bytes()).unwrap();
    }
    // Partition 2 syncs internally
    for i in 3..6 {
        for j in (i+1)..6 {
            let (left, right) = nodes.split_at_mut(j);
            sync_pair(&mut left[i], &mut right[0]);
        }
    }

    // Assert disjoint partitions have 10 objects each
    assert_eq!(nodes[0].state.object_store.len(), 10);
    assert_eq!(nodes[3].state.object_store.len(), 10);

    // --- HEAL PARTITION: Bridge Link established between Node 2 and Node 3 ---
    {
        let (left, right) = nodes.split_at_mut(3);
        sync_pair(&mut left[2], &mut right[0]);
    }

    // Gossip convergence rounds across all 6 nodes
    for _ in 0..3 {
        for i in 0..6 {
            for j in (i+1)..6 {
                let (left, right) = nodes.split_at_mut(j);
                sync_pair(&mut left[i], &mut right[0]);
            }
        }
    }

    // Assert all 6 nodes have converged to 20 objects with bit-identical Merkle state roots
    let expected_root = nodes[0].sync_now().unwrap().body.state_root;
    for i in 0..6 {
        assert_eq!(nodes[i].state.object_store.len(), 20, "Node {} must have all 20 objects after heal", i);
        let r = nodes[i].sync_now().unwrap().body.state_root;
        assert_eq!(expected_root, r, "Node {} Merkle root must match global root", i);
        nodes[i].stop().unwrap();
    }
}

#[test]
fn test_r51_3_d_30_percent_lossy_link_and_retransmission() {
    let mut csprng = OsRng;
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut node_a = NexNode::new(tmp_a.path(), key_a);
    let mut node_b = NexNode::new(tmp_b.path(), key_b);
    node_a.start().unwrap();
    node_b.start().unwrap();

    // Node A authors 30 objects
    for i in 0..30 {
        let mut meta = BTreeMap::new();
        meta.insert("idx".to_string(), format!("{}", i));
        node_a.create_object([0x01; 32], ObjectType::Synthetic(1), meta, format!("DATA_{}", i).into_bytes()).unwrap();
    }

    let mut rng = rand::thread_rng();
    let session_id = [0xDD; 16];

    // Sync under simulated 30% packet loss with retransmissions
    let mut rounds = 0;
    while node_b.state.object_store.len() < 30 && rounds < 20 {
        rounds += 1;
        let adv_b = AntiEntropyEngine::generate_advertise(&mut node_b, session_id);
        let batches = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b.frontier_mutation_ids, 5);

        for batch in batches {
            // Simulate 30% drop rate
            if rng.gen_bool(0.30) {
                // Packet dropped on wire
                continue;
            }
            let _ = AntiEntropyEngine::ingest_batch(&mut node_b, batch);
        }
    }

    assert_eq!(node_b.state.object_store.len(), 30, "Node B must recover all 30 objects through retransmission");
    let root_a = node_a.sync_now().unwrap().body.state_root;
    let root_b = node_b.sync_now().unwrap().body.state_root;
    assert_eq!(root_a, root_b, "Merkle roots must be identical despite 30% lossy links");

    node_a.stop().unwrap();
    node_b.stop().unwrap();
}

#[test]
fn test_r51_3_e_rapid_peer_churn_and_intermittent_availability() {
    let mut csprng = OsRng;
    let mut tmp_dirs = Vec::new();
    let mut nodes = Vec::new();

    for _ in 0..4 {
        let tmp = tempdir().unwrap();
        let key = SigningKey::generate(&mut csprng);
        let mut node = NexNode::new(tmp.path(), key);
        node.start().unwrap();
        tmp_dirs.push(tmp);
        nodes.push(node);
    }

    let mut rng = rand::thread_rng();

    // 5 rounds of churn: random pairs author objects and sync
    for r in 0..5 {
        let author_idx = rng.gen_range(0..4);
        let mut meta = BTreeMap::new();
        meta.insert("round".to_string(), format!("{}", r));
        nodes[author_idx].create_object([0x99; 32], ObjectType::Synthetic(1), meta, format!("CHURN_R{}_N{}", r, author_idx).into_bytes()).unwrap();

        let p1 = rng.gen_range(0..4);
        let mut p2 = rng.gen_range(0..4);
        while p1 == p2 {
            p2 = rng.gen_range(0..4);
        }

        let (min_p, max_p) = if p1 < p2 { (p1, p2) } else { (p2, p1) };
        let (left, right) = nodes.split_at_mut(max_p);
        sync_pair(&mut left[min_p], &mut right[0]);
    }

    // Stabilization phase: full pairwise sync across all 4 nodes until convergence
    for _ in 0..10 {
        for i in 0..4 {
            for j in (i+1)..4 {
                let (left, right) = nodes.split_at_mut(j);
                sync_pair(&mut left[i], &mut right[0]);
            }
        }
    }

    let target_root = nodes[0].sync_now().unwrap().body.state_root;
    for i in 0..4 {
        assert_eq!(nodes[i].state.object_store.len(), 5, "All nodes must have all 5 objects");
        let r = nodes[i].sync_now().unwrap().body.state_root;
        assert_eq!(target_root, r, "Node {} Merkle root must match stabilized root", i);
        nodes[i].stop().unwrap();
    }
}

#[test]
fn test_r51_3_f_pathological_dag_diamond_and_multiparent_convergence() {
    let mut csprng = OsRng;
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();
    let tmp_c = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::generate(&mut csprng));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::generate(&mut csprng));
    let mut node_c = NexNode::new(tmp_c.path(), SigningKey::generate(&mut csprng));

    node_a.start().unwrap();
    node_b.start().unwrap();
    node_c.start().unwrap();

    // Common root
    let mut meta = BTreeMap::new();
    meta.insert("type".to_string(), "root".to_string());
    node_a.create_object([0x01; 32], ObjectType::Synthetic(1), meta, b"ROOT".to_vec()).unwrap();
    sync_pair(&mut node_a, &mut node_b);
    sync_pair(&mut node_a, &mut node_c);

    // Node A and Node B branch concurrently (forming diamond base)
    node_a.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"BRANCH_A".to_vec()).unwrap();
    node_b.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"BRANCH_B".to_vec()).unwrap();

    // Node C syncs both branches, merging them into a multi-parent diamond head
    sync_pair(&mut node_c, &mut node_a);
    sync_pair(&mut node_c, &mut node_b);
    node_c.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"MERGED_HEAD".to_vec()).unwrap();

    // Propagate merged head back to A and B
    sync_pair(&mut node_a, &mut node_c);
    sync_pair(&mut node_b, &mut node_c);

    let root_a = node_a.sync_now().unwrap().body.state_root;
    let root_b = node_b.sync_now().unwrap().body.state_root;
    let root_c = node_c.sync_now().unwrap().body.state_root;

    assert_eq!(root_a, root_b);
    assert_eq!(root_b, root_c);
    assert_eq!(node_a.state.object_store.len(), 4);

    node_a.stop().unwrap();
    node_b.stop().unwrap();
    node_c.stop().unwrap();
}
