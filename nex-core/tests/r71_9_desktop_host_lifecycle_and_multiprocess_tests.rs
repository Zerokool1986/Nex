use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::desktop::{DesktopHostCoordinator, DesktopLifecycleState};
use nex_core::object::types::{ObjectType, NexObject};
use std::collections::BTreeMap;

#[test]
fn test_r71_9_a_desktop_host_start_and_running_state() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x11u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);

    let mut coordinator = DesktopHostCoordinator::new("NEX Desktop", temp_dir.path().to_path_buf());
    assert_eq!(coordinator.state, DesktopLifecycleState::Uninitialized);
    assert_eq!(coordinator.active_windows, 0);

    coordinator.on_app_start(&mut node).expect("App start failed");
    assert_eq!(coordinator.state, DesktopLifecycleState::Running);
    assert_eq!(coordinator.active_windows, 1);
}

#[test]
fn test_r71_9_b_multi_window_open_and_background_tray_transition() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x22u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);

    let mut coordinator = DesktopHostCoordinator::new("NEX Desktop", temp_dir.path().to_path_buf());
    coordinator.on_app_start(&mut node).unwrap();

    // Open auxiliary window (e.g. Settings / Drive Explorer)
    coordinator.on_window_opened();
    assert_eq!(coordinator.active_windows, 2);

    // Close window 1
    assert_eq!(coordinator.on_window_closed(), DesktopLifecycleState::Running);
    assert_eq!(coordinator.active_windows, 1);

    // Close window 2 -> Transitions to BackgroundTray (remains alive in system tray)
    assert_eq!(coordinator.on_window_closed(), DesktopLifecycleState::BackgroundTray);
    assert_eq!(coordinator.active_windows, 0);

    // Reopen window from tray
    coordinator.on_window_opened();
    assert_eq!(coordinator.state, DesktopLifecycleState::Running);
    assert_eq!(coordinator.active_windows, 1);
}

#[test]
fn test_r71_9_c_clean_desktop_shutdown_and_lock_cleanup() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x33u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);

    let mut coordinator = DesktopHostCoordinator::new("NEX Desktop", temp_dir.path().to_path_buf());
    coordinator.on_app_start(&mut node).unwrap();

    coordinator.on_app_stop(&mut node).expect("Clean stop failed");
    assert_eq!(coordinator.state, DesktopLifecycleState::Terminated);
    assert_eq!(coordinator.active_windows, 0);

    // Lockfile must be cleanly removed
    assert!(!temp_dir.path().join(".nex.lock").exists());
}

#[test]
fn test_r71_9_d_single_instance_duplicate_launch_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().to_path_buf();
    let seed = [0x44u8; 32];

    let signing_key1 = SigningKey::from_bytes(&seed);
    let mut node1 = NexNode::new(db_path.clone(), signing_key1);
    node1.start().expect("Node 1 start failed");

    // Attempt second node instance on same data directory
    let signing_key2 = SigningKey::from_bytes(&seed);
    let mut node2 = NexNode::new(db_path, signing_key2);
    let start2_res = node2.start();
    assert!(start2_res.is_err(), "Second instance must be rejected due to active lockfile");

    node1.stop().unwrap();
}

#[test]
fn test_r71_9_e_desktop_crash_recovery_from_wal_and_snapshot() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().to_path_buf();
    let seed = [0x55u8; 32];
    let obj_id = [0xAA; 32];

    // Process A: Writes object, checkpoints, and stops
    {
        let signing_key = SigningKey::from_bytes(&seed);
        let mut node = NexNode::new(db_path.clone(), signing_key);
        node.start().unwrap();

        let obj = NexObject {
            object_id: obj_id,
            object_type: ObjectType::VaultItem,
            namespace: [0x11; 32],
            owner_actor_id: [0x55; 32],
            schema_version: 1,
            created_epoch: 1,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: b"encrypted vault item".to_vec(),
            tombstoned: false,
        };
        node.state.object_store.insert(obj_id, obj);
        node.checkpoint_and_compact().unwrap();
        node.stop().unwrap();
    }

    // Process B (Cold Restart): Reopens and verifies data
    {
        let signing_key = SigningKey::from_bytes(&seed);
        let mut node = NexNode::new(db_path, signing_key);
        node.start().unwrap();

        let recovered = node.state.object_store.get(&obj_id).expect("Data must survive restart");
        assert_eq!(recovered.payload_bytes, b"encrypted vault item");
        node.stop().unwrap();
    }
}

#[test]
fn test_r71_9_f_forced_termination_pid_cleanup_simulation() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().to_path_buf();
    let seed = [0x66u8; 32];
    let obj_id = [0xBB; 32];

    // Process writes data and is dropped without clean stop (simulating SIGKILL)
    {
        let signing_key = SigningKey::from_bytes(&seed);
        let mut node = NexNode::new(db_path.clone(), signing_key);
        node.start().unwrap();

        let obj = NexObject {
            object_id: obj_id,
            object_type: ObjectType::DriveInode,
            namespace: [0x22; 32],
            owner_actor_id: [0x66; 32],
            schema_version: 1,
            created_epoch: 1,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: b"fsynced drive payload".to_vec(),
            tombstoned: false,
        };
        node.state.object_store.insert(obj_id, obj);
        node.checkpoint_and_compact().unwrap();
    }

    // New process starts after dead process lockfile is removed by OS
    {
        let _ = std::fs::remove_file(db_path.join(".nex.lock"));
        let signing_key = SigningKey::from_bytes(&seed);
        let mut node = NexNode::new(db_path, signing_key);
        node.start().unwrap();

        let recovered = node.state.object_store.get(&obj_id).unwrap();
        assert_eq!(recovered.payload_bytes, b"fsynced drive payload");
        node.stop().unwrap();
    }
}
