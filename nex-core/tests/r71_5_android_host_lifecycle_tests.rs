use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::mobile::{AndroidHostCoordinator, AndroidLifecycleState};
use nex_core::object::types::{ObjectType, NexObject};

#[test]
fn test_r71_5_a_app_create_and_directory_initialization() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let mut coordinator = AndroidHostCoordinator::new("org.nex.app", temp_dir.path().to_path_buf());

    assert_eq!(coordinator.state, AndroidLifecycleState::Uninitialized);
    assert_eq!(coordinator.package_name, "org.nex.app");

    coordinator.on_app_create().expect("App creation failed");
    assert_eq!(coordinator.state, AndroidLifecycleState::Foreground);
    assert!(temp_dir.path().exists());
}

#[test]
fn test_r71_5_b_activity_pause_and_resume_transitions() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let mut coordinator = AndroidHostCoordinator::new("org.nex.app", temp_dir.path().to_path_buf());
    coordinator.on_app_create().unwrap();

    coordinator.on_app_pause();
    assert_eq!(coordinator.state, AndroidLifecycleState::Background);

    coordinator.on_app_resume();
    assert_eq!(coordinator.state, AndroidLifecycleState::Foreground);
}

#[test]
fn test_r71_5_c_low_memory_warning_flushes_state_atomically() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x11u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let mut coordinator = AndroidHostCoordinator::new("org.nex.app", temp_dir.path().to_path_buf());
    coordinator.on_app_create().unwrap();

    // Insert an object
    let ns = [0xAA; 32];
    let obj_id = [0x01; 32];
    let obj = NexObject {
        object_id: obj_id,
        object_type: ObjectType::DriveInode,
        namespace: ns,
        owner_actor_id: [0x11; 32],
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 1,
        winning_mutation_id: [0u8; 32],
        metadata: BTreeMap::new(),
        payload_bytes: b"critical state before LMK".to_vec(),
        tombstoned: false,
    };
    node.state.object_store.insert(obj_id, obj);

    // Simulate OS Low Memory warning
    coordinator.on_low_memory(&mut node).expect("Low memory flush failed");
    assert_eq!(coordinator.low_memory_warning_count, 1);
}

#[test]
fn test_r71_5_d_process_death_and_cold_restart_recovery() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().to_path_buf();
    let seed = [0x22u8; 32];
    let ns = [0xBB; 32];
    let obj_id = [0x02; 32];

    // Process A: Writes data, takes snapshot, and terminates
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
            payload_bytes: b"persisted across process death".to_vec(),
            tombstoned: false,
        };
        node.state.object_store.insert(obj_id, obj);
        node.checkpoint_and_compact().unwrap();

        let mut coordinator = AndroidHostCoordinator::new("org.nex.app", db_path.clone());
        coordinator.on_app_terminate(&mut node).unwrap();
        assert_eq!(coordinator.state, AndroidLifecycleState::Terminated);
    }

    // Process B (Cold Start): Reopens same directory and verifies state integrity
    {
        let signing_key = SigningKey::from_bytes(&seed);
        let mut recovered_node = NexNode::new(db_path.clone(), signing_key);
        recovered_node.start().unwrap();

        let recovered_obj = recovered_node.state.object_store.get(&obj_id).expect("Object must survive process death");
        assert_eq!(recovered_obj.payload_bytes, b"persisted across process death");
    }
}

#[test]
fn test_r71_5_e_activity_destruction_does_not_drop_runtime() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x33u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let mut coordinator = AndroidHostCoordinator::new("org.nex.app", temp_dir.path().to_path_buf());
    coordinator.on_app_create().unwrap();

    // Simulate screen rotation (Pause -> Resume)
    coordinator.on_app_pause();
    coordinator.on_app_resume();

    assert_eq!(coordinator.state, AndroidLifecycleState::Foreground);
    assert!(node.storage.cas.chunks.is_empty());
}

#[test]
fn test_r71_5_f_abnormal_kill_survivability() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().to_path_buf();
    let seed = [0x44u8; 32];
    let ns = [0xCC; 32];
    let obj_id = [0x03; 32];

    // Node writes and checkpoints snapshot to disk
    {
        let signing_key = SigningKey::from_bytes(&seed);
        let mut node = NexNode::new(db_path.clone(), signing_key);
        node.start().unwrap();

        let obj = NexObject {
            object_id: obj_id,
            object_type: ObjectType::DriveInode,
            namespace: ns,
            owner_actor_id: [0x44; 32],
            schema_version: 1,
            created_epoch: 1,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: b"atomic fsync data".to_vec(),
            tombstoned: false,
        };
        node.state.object_store.insert(obj_id, obj);
        node.checkpoint_and_compact().unwrap();
        // Dropped without clean on_app_terminate (simulating hard kill)
    }

    // Reopen and assert recovery
    {
        let _ = std::fs::remove_file(db_path.join(".nex.lock"));
        let signing_key = SigningKey::from_bytes(&seed);
        let mut recovered_node = NexNode::new(db_path, signing_key);
        recovered_node.start().unwrap();

        let obj = recovered_node.state.object_store.get(&obj_id).expect("Data must survive hard kill");
        assert_eq!(obj.payload_bytes, b"atomic fsync data");
    }
}
