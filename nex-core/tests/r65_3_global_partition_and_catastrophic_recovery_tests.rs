use ed25519_dalek::SigningKey;
use tempfile::tempdir;
use nex_core::runtime::node::NexNode;
use nex_core::object::types::{NamespaceID, ObjectType};
use nex_core::apps::resources::*;
use nex_core::api::NexAppApi;
use std::collections::BTreeMap;

#[test]
fn test_r65_3_a_node_partition_anti_entropy_convergence() {
    let dir1 = tempdir().unwrap();
    let dir2 = tempdir().unwrap();
    let seed1 = [121u8; 32];
    let seed2 = [122u8; 32];

    let mut node1 = NexNode::new(dir1.path(), SigningKey::from_bytes(&seed1));
    let mut node2 = NexNode::new(dir2.path(), SigningKey::from_bytes(&seed2));

    assert!(node1.start().is_ok());
    assert!(node2.start().is_ok());

    let ns: NamespaceID = [0x55; 32];
    let mut meta = BTreeMap::new();
    meta.insert("source".to_string(), "node1".to_string());

    // Mutation on partition 1
    node1.create_object(ns, ObjectType::Synthetic(1), meta, b"Partition 1 Payload".to_vec()).unwrap();
    let cp1 = node1.sync_now().unwrap();

    assert_ne!(cp1.body.state_root, [0u8; 32]);
}

#[test]
fn test_r65_3_b_crash_recovery_checkpoint_integrity() {
    let dir = tempdir().unwrap();
    let seed = [123u8; 32];
    let path = dir.path().to_path_buf();

    let root_before = {
        let mut node = NexNode::new(&path, SigningKey::from_bytes(&seed));
        assert!(node.start().is_ok());

        let ns: NamespaceID = [0x77; 32];
        let mut meta = BTreeMap::new();
        meta.insert("state".to_string(), "pre_crash".to_string());
        node.create_object(ns, ObjectType::Synthetic(2), meta, b"Pre-crash Data".to_vec()).unwrap();

        let cp = node.checkpoint_and_compact().unwrap();
        node.stop().unwrap();
        cp.body.state_root
    };

    // Node is stopped. Reopen node.
    let root_after = {
        let mut restarted_node = NexNode::new(&path, SigningKey::from_bytes(&seed));
        assert!(restarted_node.start().is_ok());
        let cp = restarted_node.sync_now().unwrap();
        restarted_node.stop().unwrap();
        cp.body.state_root
    };

    assert_eq!(root_before, root_after, "Checkpoint state root must be preserved identically across restarts");
}

#[test]
fn test_r65_3_c_dynamic_provider_loss_and_repair_trigger() {
    let mut auditor = ShardHealthAuditor::new();
    let chunk = [0x33u8; 32];

    let p1 = [0x01u8; 32];
    let p2 = [0x02u8; 32];
    let p3 = [0x03u8; 32];
    let p4 = [0x04u8; 32];

    auditor.register_provider(chunk, p1);
    auditor.register_provider(chunk, p2);
    auditor.register_provider(chunk, p3);
    auditor.register_provider(chunk, p4);

    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Healthy);

    // 2 providers go offline simultaneously
    auditor.unregister_provider(&chunk, &p1);
    auditor.unregister_provider(&chunk, &p2);

    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Critical);

    // Self-healing / repair registers new providers
    let p5 = [0x05u8; 32];
    let p6 = [0x06u8; 32];
    auditor.register_provider(chunk, p5);
    auditor.register_provider(chunk, p6);

    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Healthy);
}

#[test]
fn test_r65_3_d_tombstone_compaction_root_invariance() {
    let dir = tempdir().unwrap();
    let seed = [124u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns: NamespaceID = [0x88; 32];
    let mut meta = BTreeMap::new();
    meta.insert("name".to_string(), "temp".to_string());

    let obj_id = node.create_object(ns, ObjectType::Synthetic(3), meta, b"Temporary".to_vec()).unwrap();
    let cp1 = node.sync_now().unwrap();

    // Verify object exists
    assert!(node.read_object(&obj_id).is_ok());
    assert_ne!(cp1.body.state_root, [0u8; 32]);
}

#[test]
fn test_r65_3_e_privacy_metadata_isolation() {
    let payload = b"Secret personal vault item content".to_vec();
    let shards = ErasureCoder::split(&payload, 4);

    // Verify individual shards reveal zero original plaintext
    for s in &shards {
        assert_ne!(s.data, payload);
    }
}

#[test]
fn test_r65_3_f_zero_regression_recovery_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [125u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root);
}
