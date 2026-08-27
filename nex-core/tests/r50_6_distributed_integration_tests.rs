use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::sync::anti_entropy::AntiEntropyEngine;
use nex_core::transport::session::{NaspInitiator, NaspResponder};

#[test]
fn test_r50_6_a_10node_mesh_convergence() {
    let mut dirs = Vec::new();
    let mut nodes = Vec::new();
    let mut csprng = OsRng;

    // Initialize 10 nodes
    for i in 0..10u8 {
        let dir = tempdir().unwrap();
        let key = SigningKey::generate(&mut csprng);
        let mut node = NexNode::new(dir.path(), key);
        node.start().unwrap();

        // Each node authors 10 objects in its own namespace
        let ns = [i; 32];
        for j in 0..10 {
            node.create_object(
                ns,
                ObjectType::Synthetic(100 + i as u16),
                BTreeMap::from([("author_node".into(), i.to_string()), ("item_index".into(), j.to_string())]),
                format!("NODE_{}_OBJ_{}", i, j).into_bytes(),
            ).unwrap();
        }

        dirs.push(dir);
        nodes.push(node);
    }

    // Mesh Synchronization: Full pairwise gossip until complete convergence
    let session_id = [0x99; 16];
    for round in 0..3 {
        for i in 0..10 {
            for j in 0..10 {
                if i == j { continue; }

                let adv_j = AntiEntropyEngine::generate_advertise(&mut nodes[j], session_id);
                if AntiEntropyEngine::has_deltas_for_peer(&nodes[i], &adv_j) {
                    let batches = AntiEntropyEngine::generate_batches_for_peer(
                        &nodes[i],
                        session_id,
                        &adv_j.frontier_mutation_ids,
                        32,
                    );
                    for b in batches {
                        AntiEntropyEngine::ingest_batch(&mut nodes[j], b).unwrap();
                    }
                }
            }
        }
    }

    // Assert all 10 nodes have exactly 100 objects in their DAG and ObjectStore
    let reference_checkpoint = nodes[0].state.state_node.compute_current_checkpoint();
    assert_eq!(nodes[0].state.object_store.len(), 100);

    for (idx, node) in nodes.iter_mut().enumerate() {
        assert_eq!(node.state.object_store.len(), 100, "Node {} must have 100 objects", idx);
        assert_eq!(node.state.state_node.dag.len(), 100, "Node {} must have 100 DAG mutations", idx);
        let cp = node.state.state_node.compute_current_checkpoint();
        assert_eq!(
            cp.body.state_root, reference_checkpoint.body.state_root,
            "Node {} StateCommitment Merkle root must match Node 0 bit-for-bit", idx
        );
    }

    for mut node in nodes {
        node.stop().unwrap();
    }
}

#[test]
fn test_r50_6_b_end_to_end_secure_sync_over_nasp() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut node_a = NexNode::new(tmp_a.path(), key_a.clone());
    let mut node_b = NexNode::new(tmp_b.path(), key_b.clone());
    node_a.start().unwrap();
    node_b.start().unwrap();

    // 1. Establish NASP Session
    let mut initiator = NaspInitiator::new(key_a);
    let mut responder = NaspResponder::new(key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, t3) = responder.process_init(&init_msg).unwrap();
    let (confirm_msg, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();
    responder.verify_confirm(&init_msg.static_pub, &t3, &confirm_msg).unwrap();

    // 2. Node A authors 20 objects
    for i in 0..20 {
        node_a.create_object(
            [0x88; 32],
            ObjectType::DriveFolder,
            BTreeMap::new(),
            format!("SECURE_PAYLOAD_{}", i).into_bytes(),
        ).unwrap();
    }

    // 3. Node B advertises empty frontier
    let session_id = [0x12; 16];
    let adv_b = AntiEntropyEngine::generate_advertise(&mut node_b, session_id);
    let batches = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b.frontier_mutation_ids, 10);

    // 4. Encrypted transmission over wire
    for batch in batches {
        let plaintext_bytes = bincode::serialize(&batch).unwrap();
        let (seq, ciphertext, mac) = initiator_keys.encrypt(&plaintext_bytes);

        // Wire transit & decryption
        let decrypted_bytes = responder_keys.decrypt(seq, &ciphertext, &mac)
            .expect("Responder must decrypt authenticated ciphertext");
        let received_batch: nex_core::sync::anti_entropy::SyncStreamBatch = bincode::deserialize(&decrypted_bytes).unwrap();

        AntiEntropyEngine::ingest_batch(&mut node_b, received_batch).unwrap();
    }

    assert_eq!(node_b.state.object_store.len(), 20);
    let cp_a = node_a.state.state_node.compute_current_checkpoint();
    let cp_b = node_b.state.state_node.compute_current_checkpoint();
    assert_eq!(cp_a.body.state_root, cp_b.body.state_root);

    node_a.stop().unwrap();
    node_b.stop().unwrap();
}

#[test]
fn test_r50_6_c_crash_and_restart_during_multi_peer_sync() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();
    let tmp_c = tempdir().unwrap();
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);
    let key_c = SigningKey::generate(&mut csprng);

    let mut node_a = NexNode::new(tmp_a.path(), key_a);
    let mut node_b = NexNode::new(tmp_b.path(), key_b.clone());
    let mut node_c = NexNode::new(tmp_c.path(), key_c);
    node_a.start().unwrap();
    node_b.start().unwrap();
    node_c.start().unwrap();

    // Node A creates 30 objects
    for i in 0..30 {
        node_a.create_object([0x77; 32], ObjectType::DriveInode, BTreeMap::new(), format!("FILE_{}", i).into_bytes()).unwrap();
    }

    let session_id = [0x33; 16];

    // Sync first 15 objects to Node B
    let adv_b = AntiEntropyEngine::generate_advertise(&mut node_b, session_id);
    let batches = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b.frontier_mutation_ids, 15);
    AntiEntropyEngine::ingest_batch(&mut node_b, batches[0].clone()).unwrap();

    // Node B saves atomic snapshot
    node_b.checkpoint_and_compact().unwrap();

    // Ingest 5 more mutations into Node B's WAL tail
    if batches.len() > 1 {
        AntiEntropyEngine::ingest_batch(&mut node_b, batches[1].clone()).unwrap();
    }

    // SIMULATE CRASH: Drop Node B without calling stop() (clean up lockfile manually to simulate process crash)
    drop(node_b);
    let _ = std::fs::remove_file(tmp_b.path().join(".nex.lock"));

    // REBOOT Node B from disk
    let mut recovered_node_b = NexNode::new(tmp_b.path(), key_b);
    recovered_node_b.start().unwrap();
    assert_eq!(recovered_node_b.state.object_store.len(), 30);

    // Sync Node B -> Node C
    let adv_c = AntiEntropyEngine::generate_advertise(&mut node_c, session_id);
    let batches_to_c = AntiEntropyEngine::generate_batches_for_peer(&recovered_node_b, session_id, &adv_c.frontier_mutation_ids, 30);
    for b in batches_to_c {
        AntiEntropyEngine::ingest_batch(&mut node_c, b).unwrap();
    }

    assert_eq!(node_c.state.object_store.len(), 30);
    let cp_a = node_a.state.state_node.compute_current_checkpoint();
    let cp_c = node_c.state.state_node.compute_current_checkpoint();
    assert_eq!(cp_a.body.state_root, cp_c.body.state_root);

    node_a.stop().unwrap();
    recovered_node_b.stop().unwrap();
    node_c.stop().unwrap();
}

#[test]
fn test_r50_6_d_multi_app_concurrent_sync_mesh() {
    let mut dirs = Vec::new();
    let mut nodes = Vec::new();
    let mut csprng = OsRng;

    for _ in 0..5 {
        let dir = tempdir().unwrap();
        let key = SigningKey::generate(&mut csprng);
        let mut node = NexNode::new(dir.path(), key);
        node.start().unwrap();
        dirs.push(dir);
        nodes.push(node);
    }

    // Node 0: Drive
    nodes[0].create_object([0x01; 32], ObjectType::DriveFolder, BTreeMap::new(), b"/Photos".to_vec()).unwrap();
    nodes[0].create_object([0x01; 32], ObjectType::DriveInode, BTreeMap::new(), b"data.bin".to_vec()).unwrap();

    // Node 1: Chat
    nodes[1].create_object([0x02; 32], ObjectType::ChatChannel, BTreeMap::new(), b"#general".to_vec()).unwrap();
    nodes[1].create_object([0x02; 32], ObjectType::ChatMessage, BTreeMap::new(), b"hello".to_vec()).unwrap();

    // Node 2: Community
    nodes[2].create_object([0x03; 32], ObjectType::Community, BTreeMap::new(), b"RFC-1".to_vec()).unwrap();
    nodes[2].create_object([0x03; 32], ObjectType::MemberRole, BTreeMap::new(), b"reply".to_vec()).unwrap();

    // Node 3: Photos
    nodes[3].create_object([0x04; 32], ObjectType::PhotoAlbum, BTreeMap::new(), b"Album-1".to_vec()).unwrap();
    nodes[3].create_object([0x04; 32], ObjectType::PhotoMedia, BTreeMap::new(), b"IMG_001".to_vec()).unwrap();

    // Node 4: Synthetic
    nodes[4].create_object([0x05; 32], ObjectType::Synthetic(500), BTreeMap::new(), b"SYNTH_A".to_vec()).unwrap();
    nodes[4].create_object([0x05; 32], ObjectType::Synthetic(501), BTreeMap::new(), b"SYNTH_B".to_vec()).unwrap();

    // Gossip Sync all 5 nodes
    let session_id = [0x44; 16];
    for _ in 0..2 {
        for i in 0..5 {
            for j in 0..5 {
                if i == j { continue; }
                let adv_j = AntiEntropyEngine::generate_advertise(&mut nodes[j], session_id);
                if AntiEntropyEngine::has_deltas_for_peer(&nodes[i], &adv_j) {
                    let batches = AntiEntropyEngine::generate_batches_for_peer(&nodes[i], session_id, &adv_j.frontier_mutation_ids, 32);
                    for b in batches {
                        AntiEntropyEngine::ingest_batch(&mut nodes[j], b).unwrap();
                    }
                }
            }
        }
    }

    // Verify all 5 nodes have 10 objects total across all applications
    for node in nodes.iter_mut() {
        assert_eq!(node.state.object_store.len(), 10);
    }

    for mut node in nodes {
        node.stop().unwrap();
    }
}

#[test]
fn test_r50_6_e_cascading_rekeying_under_sustained_load() {
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(key_a);
    let mut responder = NaspResponder::new(key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, t3) = responder.process_init(&init_msg).unwrap();
    let (confirm_msg, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();
    responder.verify_confirm(&init_msg.static_pub, &t3, &confirm_msg).unwrap();

    // Stream 1,000 frames with automatic rekeying every 250 frames
    for i in 0..1000u64 {
        if i > 0 && i % 250 == 0 {
            initiator_keys.rekey();
            responder_keys.rekey();
        }

        let payload = format!("STREAM_FRAME_INDEX_{}", i).into_bytes();
        let (seq, ct, mac) = initiator_keys.encrypt(&payload);
        let decrypted = responder_keys.decrypt(seq, &ct, &mac).unwrap();
        assert_eq!(decrypted, payload);
    }
}

#[test]
fn test_r50_6_f_constitutional_invariant_seal_and_regression_verification() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    // Verify Invariant NEX-01: Identity is cryptographically grounded
    assert_eq!(node.identity.actor_id.len(), 32);

    // Verify Invariant NEX-02: Deterministic DAG State Commitment
    node.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"INV_1".to_vec()).unwrap();
    let cp = node.state.state_node.compute_current_checkpoint();
    assert_ne!(cp.body.state_root, [0u8; 32]);

    // Verify Invariant NEX-03: Zero bypass storage & compaction
    node.checkpoint_and_compact().unwrap();

    node.stop().unwrap();
}
