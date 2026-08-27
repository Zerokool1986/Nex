use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::sync::outbox::{OfflineOutboxStore, OutboxEntry};
use nex_core::object::types::{ObjectType, NexObject};

#[test]
fn test_r71_18_a_enqueue_and_drain_pending_outbox() {
    let mut outbox = OfflineOutboxStore::new();

    let e1 = OutboxEntry {
        entry_id: [0x01; 16],
        object_id: [0x11; 32],
        namespace: [0xAA; 32],
        target_peer: None,
        payload_bytes: b"Outbox item 1".to_vec(),
        enqueued_epoch: 10,
        attempts: 0,
        acknowledged: false,
    };
    let e2 = OutboxEntry {
        entry_id: [0x02; 16],
        object_id: [0x22; 32],
        namespace: [0xBB; 32],
        target_peer: None,
        payload_bytes: b"Outbox item 2".to_vec(),
        enqueued_epoch: 10,
        attempts: 0,
        acknowledged: false,
    };

    outbox.enqueue(e1.clone());
    outbox.enqueue(e2.clone());

    assert_eq!(outbox.pending_entries().len(), 2);

    // Acknowledge item 1
    assert!(outbox.acknowledge(&e1.entry_id));
    assert_eq!(outbox.pending_entries().len(), 1);
    assert_eq!(outbox.pending_entries()[0].entry_id, e2.entry_id);
}

#[test]
fn test_r71_18_b_record_delivery_failure_increments_attempts() {
    let mut outbox = OfflineOutboxStore::new();
    let entry_id = [0x03; 16];

    outbox.enqueue(OutboxEntry {
        entry_id,
        object_id: [0x33; 32],
        namespace: [0xCC; 32],
        target_peer: None,
        payload_bytes: b"Retry payload".to_vec(),
        enqueued_epoch: 10,
        attempts: 0,
        acknowledged: false,
    });

    outbox.record_failure(&entry_id);
    outbox.record_failure(&entry_id);

    let pending = outbox.pending_entries();
    assert_eq!(pending[0].attempts, 2);
}

#[test]
fn test_r71_18_c_outbox_recovery_from_uncompacted_node_state() {
    let temp_dir = tempdir().unwrap();
    let mut node = NexNode::new(temp_dir.path(), SigningKey::from_bytes(&[0x11; 32]));
    node.start().unwrap();

    let obj_id = [0x44; 32];
    node.state.object_store.insert(obj_id, NexObject {
        object_id: obj_id,
        object_type: ObjectType::DriveInode,
        namespace: [0xDD; 32],
        owner_actor_id: [0x11; 32],
        schema_version: 1,
        created_epoch: 15,
        created_lamport: 15,
        winning_mutation_id: [0u8; 32],
        metadata: BTreeMap::new(),
        payload_bytes: b"Recoverable state".to_vec(),
        tombstoned: false,
    });

    let mut outbox = OfflineOutboxStore::new();
    outbox.recover_from_node_state(&node);

    assert_eq!(outbox.pending_entries().len(), 1);
    assert_eq!(outbox.pending_entries()[0].object_id, obj_id);
}

#[test]
fn test_r71_18_d_duplicate_enqueue_is_idempotent() {
    let mut outbox = OfflineOutboxStore::new();
    let entry_id = [0x05; 16];

    let entry = OutboxEntry {
        entry_id,
        object_id: [0x55; 32],
        namespace: [0xEE; 32],
        target_peer: None,
        payload_bytes: b"Deduplicated".to_vec(),
        enqueued_epoch: 10,
        attempts: 0,
        acknowledged: false,
    };

    outbox.enqueue(entry.clone());
    outbox.enqueue(entry);

    assert_eq!(outbox.entries.len(), 1);
}

#[test]
fn test_r71_18_e_nonexistent_ack_returns_false() {
    let mut outbox = OfflineOutboxStore::new();
    assert!(!outbox.acknowledge(&[0xFF; 16]));
}

#[test]
fn test_r71_18_f_process_restart_preserves_outbox_queue() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().to_path_buf();
    let seed = [0x22u8; 32];
    let obj_id = [0x66; 32];

    // Node creates state and checkpoints
    {
        let mut node = NexNode::new(db_path.clone(), SigningKey::from_bytes(&seed));
        node.start().unwrap();
        node.state.object_store.insert(obj_id, NexObject {
            object_id: obj_id,
            object_type: ObjectType::VaultItem,
            namespace: [0x01; 32],
            owner_actor_id: [0x22; 32],
            schema_version: 1,
            created_epoch: 1,
            created_lamport: 1,
            winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: b"Durable outbox mutation".to_vec(),
            tombstoned: false,
        });
        node.checkpoint_and_compact().unwrap();
        node.stop().unwrap();
    }

    // Process restarts and reconstructs outbox
    {
        let mut node = NexNode::new(db_path, SigningKey::from_bytes(&seed));
        node.start().unwrap();

        let mut outbox = OfflineOutboxStore::new();
        outbox.recover_from_node_state(&node);

        assert_eq!(outbox.pending_entries().len(), 1);
        assert_eq!(outbox.pending_entries()[0].payload_bytes, b"Durable outbox mutation");
    }
}
