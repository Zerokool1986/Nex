use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::mobile::{AndroidWorkManagerScheduler, DevicePowerState};
use nex_core::object::types::{ObjectType, NexObject};

#[test]
fn test_r71_7_a_workmanager_scheduler_enqueue_and_drain() {
    let mut scheduler = AndroidWorkManagerScheduler::new(false, false);
    let obj1 = [0x01; 32];
    let obj2 = [0x02; 32];

    scheduler.enqueue_sync_target(obj1);
    scheduler.enqueue_sync_target(obj2);
    // Duplicate enqueue must be deduplicated
    scheduler.enqueue_sync_target(obj1);

    assert_eq!(scheduler.pending_sync_items.len(), 2);

    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x11u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let executed_count = scheduler.execute_scheduled_sync(&mut node).expect("Sync execution failed");
    assert_eq!(executed_count, 2);
    assert!(scheduler.pending_sync_items.is_empty(), "Queue must be empty after execution");
}

#[test]
fn test_r71_7_b_workmanager_charging_constraint_enforcement() {
    let scheduler = AndroidWorkManagerScheduler::new(true, false); // requires charging

    // Battery discharging
    assert!(!scheduler.can_execute(false, false, DevicePowerState::Active));
    // Battery plugged in
    assert!(scheduler.can_execute(true, false, DevicePowerState::Active));
}

#[test]
fn test_r71_7_c_workmanager_unmetered_network_constraint() {
    let scheduler = AndroidWorkManagerScheduler::new(false, true); // requires unmetered (WiFi)

    // Cellular data (metered)
    assert!(!scheduler.can_execute(false, false, DevicePowerState::Active));
    // WiFi (unmetered)
    assert!(scheduler.can_execute(false, true, DevicePowerState::Active));
}

#[test]
fn test_r71_7_d_workmanager_doze_suppression() {
    let scheduler = AndroidWorkManagerScheduler::new(false, false);

    // In deep Doze, sync must not run
    assert!(!scheduler.can_execute(true, true, DevicePowerState::DozeStandby));
    // When device wakes up, sync is permitted
    assert!(scheduler.can_execute(true, true, DevicePowerState::Active));
}

#[test]
fn test_r71_7_e_idempotent_sync_resume_after_interruption() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().to_path_buf();
    let seed = [0x22u8; 32];
    let ns = [0xDD; 32];
    let obj_id = [0x04; 32];

    // Node puts object and performs scheduled sync
    {
        let signing_key = SigningKey::from_bytes(&seed);
        let mut node = NexNode::new(db_path.clone(), signing_key);
        node.start().unwrap();

        let obj = NexObject {
            object_id: obj_id,
            object_type: ObjectType::DriveInode,
            namespace: ns,
            owner_actor_id: [0x22; 32],
            schema_version: 1,
            created_epoch: 1,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: b"large chunk payload to synchronize".to_vec(),
            tombstoned: false,
        };
        node.state.object_store.insert(obj_id, obj);
        node.checkpoint_and_compact().unwrap();

        let mut scheduler = AndroidWorkManagerScheduler::new(false, false);
        scheduler.enqueue_sync_target(obj_id);
        scheduler.execute_scheduled_sync(&mut node).unwrap();
    }

    // Process restarts (simulating worker death after sync)
    {
        let _ = std::fs::remove_file(db_path.join(".nex.lock"));
        let signing_key = SigningKey::from_bytes(&seed);
        let mut recovered_node = NexNode::new(db_path, signing_key);
        recovered_node.start().unwrap();

        let mut scheduler2 = AndroidWorkManagerScheduler::new(false, false);
        scheduler2.enqueue_sync_target(obj_id);
        // Resuming sync on already synchronized object is completely idempotent
        let res = scheduler2.execute_scheduled_sync(&mut recovered_node);
        assert!(res.is_ok());
    }
}

#[test]
fn test_r71_7_f_workmanager_empty_queue_no_op() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x33u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let mut scheduler = AndroidWorkManagerScheduler::new(false, false);
    let executed_count = scheduler.execute_scheduled_sync(&mut node).unwrap();
    assert_eq!(executed_count, 0);
}
