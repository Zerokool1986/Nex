use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::sync::anti_entropy::AntiEntropyEngine;

fn sync_nodes(node_a: &mut NexNode, node_b: &mut NexNode) {
    let session_id = [0x77; 16];

    let adv_b = AntiEntropyEngine::generate_advertise(node_b, session_id);
    let batches_a_to_b = AntiEntropyEngine::generate_batches_for_peer(node_a, session_id, &adv_b.frontier_mutation_ids, 100);
    for batch in batches_a_to_b {
        let _ = AntiEntropyEngine::ingest_batch(node_b, batch);
    }

    let adv_a = AntiEntropyEngine::generate_advertise(node_a, session_id);
    let batches_b_to_a = AntiEntropyEngine::generate_batches_for_peer(node_b, session_id, &adv_a.frontier_mutation_ids, 100);
    for batch in batches_b_to_a {
        let _ = AntiEntropyEngine::ingest_batch(node_a, batch);
    }
}

#[test]
fn test_r71_17_a_two_node_offline_divergence_and_convergence() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::from_bytes(&[0x01; 32]));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x02; 32]));
    node_a.start().unwrap();
    node_b.start().unwrap();

    // Node A authors object while offline
    let mut meta_a = BTreeMap::new();
    meta_a.insert("author".to_string(), "node_a".to_string());
    node_a.create_object([0x11; 32], ObjectType::Synthetic(1), meta_a, b"Node A Payload".to_vec()).unwrap();

    // Node B authors object while offline
    let mut meta_b = BTreeMap::new();
    meta_b.insert("author".to_string(), "node_b".to_string());
    node_b.create_object([0x22; 32], ObjectType::Synthetic(1), meta_b, b"Node B Payload".to_vec()).unwrap();

    assert_eq!(node_a.state.object_store.len(), 1);
    assert_eq!(node_b.state.object_store.len(), 1);

    // Reconnect and sync
    sync_nodes(&mut node_a, &mut node_b);
    sync_nodes(&mut node_a, &mut node_b);

    assert_eq!(node_a.state.object_store.len(), 2);
    assert_eq!(node_b.state.object_store.len(), 2);

    let root_a = node_a.sync_now().unwrap().body.state_root;
    let root_b = node_b.sync_now().unwrap().body.state_root;
    assert_eq!(root_a, root_b, "State roots must converge deterministically");
}

#[test]
fn test_r71_17_b_concurrent_writes_resolve_deterministically() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::from_bytes(&[0x03; 32]));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x04; 32]));
    node_a.start().unwrap();
    node_b.start().unwrap();

    // Both author objects in same namespace
    let ns = [0x55; 32];
    node_a.create_object(ns, ObjectType::Synthetic(2), BTreeMap::new(), b"A version".to_vec()).unwrap();
    node_b.create_object(ns, ObjectType::Synthetic(2), BTreeMap::new(), b"B version".to_vec()).unwrap();

    for _ in 0..3 {
        sync_nodes(&mut node_a, &mut node_b);
    }

    let root_a = node_a.sync_now().unwrap().body.state_root;
    let root_b = node_b.sync_now().unwrap().body.state_root;
    assert_eq!(root_a, root_b);
}

#[test]
fn test_r71_17_c_3_node_transitive_convergence() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();
    let tmp_c = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::from_bytes(&[0x05; 32]));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x06; 32]));
    let mut node_c = NexNode::new(tmp_c.path(), SigningKey::from_bytes(&[0x07; 32]));
    node_a.start().unwrap();
    node_b.start().unwrap();
    node_c.start().unwrap();

    node_a.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"Item 1".to_vec()).unwrap();
    node_b.create_object([0x02; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"Item 2".to_vec()).unwrap();
    node_c.create_object([0x03; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"Item 3".to_vec()).unwrap();

    // Sync A <-> B, then B <-> C, then A <-> C
    for _ in 0..5 {
        sync_nodes(&mut node_a, &mut node_b);
        sync_nodes(&mut node_b, &mut node_c);
        sync_nodes(&mut node_a, &mut node_c);
    }

    assert_eq!(node_a.state.object_store.len(), 3);
    assert_eq!(node_b.state.object_store.len(), 3);
    assert_eq!(node_c.state.object_store.len(), 3);

    let r_a = node_a.sync_now().unwrap().body.state_root;
    let r_b = node_b.sync_now().unwrap().body.state_root;
    let r_c = node_c.sync_now().unwrap().body.state_root;
    assert_eq!(r_a, r_b);
    assert_eq!(r_b, r_c);
}

#[test]
fn test_r71_17_d_empty_sync_is_noop() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::from_bytes(&[0x08; 32]));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x09; 32]));
    node_a.start().unwrap();
    node_b.start().unwrap();

    let r_a_before = node_a.sync_now().unwrap().body.state_root;
    let r_b_before = node_b.sync_now().unwrap().body.state_root;

    sync_nodes(&mut node_a, &mut node_b);

    assert_eq!(node_a.sync_now().unwrap().body.state_root, r_a_before);
    assert_eq!(node_b.sync_now().unwrap().body.state_root, r_b_before);
}

#[test]
fn test_r71_17_e_repetition_idempotency() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::from_bytes(&[0x0A; 32]));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x0B; 32]));
    node_a.start().unwrap();
    node_b.start().unwrap();

    node_a.create_object([0x33; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"Idempotence Test".to_vec()).unwrap();

    // Sync once
    sync_nodes(&mut node_a, &mut node_b);
    let root1 = node_b.sync_now().unwrap().body.state_root;

    // Sync 5 more times with no new data
    for _ in 0..5 {
        sync_nodes(&mut node_a, &mut node_b);
    }
    let root2 = node_b.sync_now().unwrap().body.state_root;

    assert_eq!(root1, root2, "Repeated sync rounds must be completely idempotent");
}

#[test]
fn test_r71_17_f_disconnected_offline_accumulation() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::from_bytes(&[0x0C; 32]));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x0D; 32]));
    node_a.start().unwrap();
    node_b.start().unwrap();

    // Accumulate 5 objects on A
    for i in 0..5 {
        node_a.create_object([0x44; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!("Batch Item {}", i).into_bytes()).unwrap();
    }

    assert_eq!(node_a.state.object_store.len(), 5);
    assert_eq!(node_b.state.object_store.len(), 0);

    sync_nodes(&mut node_a, &mut node_b);
    sync_nodes(&mut node_a, &mut node_b);

    assert_eq!(node_b.state.object_store.len(), 5);
}
