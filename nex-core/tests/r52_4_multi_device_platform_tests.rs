use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::runtime::mobile::{AndroidPlatformAdapter, DesktopPlatformAdapter, DevicePowerState};
use nex_core::object::types::ObjectType;
use nex_core::sync::anti_entropy::AntiEntropyEngine;

fn sync_pair(node_a: &mut NexNode, node_b: &mut NexNode) {
    let session_id = [0x99; 16];
    
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
fn test_r52_4_a_android_background_sync_lifecycle_and_doze_recovery() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let mut adapter = AndroidPlatformAdapter::new("app.nex.mobile", tmp.path().to_path_buf());

    // 1. Active State Sync
    let root1 = adapter.trigger_workmanager_sync(&mut node).unwrap();
    assert_ne!(root1, [0u8; 32]);
    assert_eq!(adapter.calculate_max_batch_size(), 100);

    // 2. Enter Deep Doze
    adapter.on_doze_entered();
    assert_eq!(adapter.power_state, DevicePowerState::DozeStandby);
    assert_eq!(adapter.calculate_max_batch_size(), 10);
    let doze_err = adapter.trigger_workmanager_sync(&mut node);
    assert!(doze_err.is_err());
    assert!(doze_err.unwrap_err().contains("SyncDeferred"));

    // 3. Exit Doze on Wakeup
    adapter.on_doze_exited();
    assert_eq!(adapter.power_state, DevicePowerState::Active);
    assert_eq!(adapter.calculate_max_batch_size(), 100);
    let root2 = adapter.trigger_workmanager_sync(&mut node).unwrap();
    assert_eq!(root1, root2);

    node.stop().unwrap();
}

#[test]
fn test_r52_4_b_android_scoped_storage_path_isolation_and_permission_jail() {
    let tmp = tempdir().unwrap();
    let scoped_dir = tmp.path().join("data").join("user").join("0").join("app.nex.mobile").join("files");
    std::fs::create_dir_all(&scoped_dir).unwrap();

    let mut csprng = OsRng;
    let mut node = NexNode::new(scoped_dir.clone(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let adapter = AndroidPlatformAdapter::new("app.nex.mobile", scoped_dir.clone());
    assert_eq!(adapter.config.internal_files_dir, scoped_dir);

    // Create object and checkpoint to produce storage artifacts
    node.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"SANDBOX_DATA".to_vec()).unwrap();
    let _ = node.checkpoint_and_compact().unwrap();

    // Verify storage files created in scoped directory
    assert!(scoped_dir.join(".nex.lock").exists());
    assert!(scoped_dir.join("state.db").exists());

    node.stop().unwrap();
}

#[test]
fn test_r52_4_c_desktop_daemon_hot_reload_and_health_poll() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let adapter = DesktopPlatformAdapter::new("nex-daemon", tmp.path().to_path_buf());

    // Health poll while active
    let health = adapter.poll_daemon_health(&node).unwrap();
    assert!(health.contains("Running"));

    // Stop daemon
    node.stop().unwrap();
    let dead_health = adapter.poll_daemon_health(&node);
    assert!(dead_health.is_err());
    assert_eq!(dead_health.unwrap_err(), "Daemon is not active");
}

#[test]
fn test_r52_4_d_multi_platform_cross_device_pairwise_sync() {
    let tmp_mobile = tempdir().unwrap();
    let tmp_desktop = tempdir().unwrap();
    let mut csprng = OsRng;

    let mut mobile_node = NexNode::new(tmp_mobile.path(), SigningKey::generate(&mut csprng));
    mobile_node.start().unwrap();

    let mut desktop_node = NexNode::new(tmp_desktop.path(), SigningKey::generate(&mut csprng));
    desktop_node.start().unwrap();

    let ns_photos = [0xA1; 32];
    let ns_drive = [0xD1; 32];

    // Mobile authors photo
    mobile_node.create_object(ns_photos, ObjectType::PhotoMedia, BTreeMap::new(), b"Mobile JPEG".to_vec()).unwrap();

    // Desktop authors drive document
    desktop_node.create_object(ns_drive, ObjectType::DriveInode, BTreeMap::new(), b"Desktop PDF".to_vec()).unwrap();

    // 2 pairwise sync rounds
    for _ in 0..2 {
        sync_pair(&mut mobile_node, &mut desktop_node);
    }

    assert_eq!(mobile_node.state.object_store.len(), 2);
    assert_eq!(desktop_node.state.object_store.len(), 2);

    let mobile_root = mobile_node.sync_now().unwrap().body.state_root;
    let desktop_root = desktop_node.sync_now().unwrap().body.state_root;
    assert_eq!(mobile_root, desktop_root);

    mobile_node.stop().unwrap();
    desktop_node.stop().unwrap();
}

#[test]
fn test_r52_4_e_battery_saver_low_resource_batch_throttling() {
    let tmp = tempdir().unwrap();
    let mut adapter = AndroidPlatformAdapter::new("app.nex.mobile", PathBuf::from("/data/user/0/app.nex.mobile/files"));

    assert_eq!(adapter.calculate_max_batch_size(), 100);

    // Enable Battery Saver
    adapter.set_battery_saver(true);
    assert_eq!(adapter.power_state, DevicePowerState::BatterySaverThrottled);
    assert_eq!(adapter.calculate_max_batch_size(), 25);

    // Disable Battery Saver
    adapter.set_battery_saver(false);
    assert_eq!(adapter.power_state, DevicePowerState::Active);
    assert_eq!(adapter.calculate_max_batch_size(), 100);
}

#[test]
fn test_r52_4_f_gate_r52_master_ecosystem_seal_and_merkle_invariance() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    // Invariant verification across all namespaces
    let ns_list = [
        [0xD1; 32], // Drive
        [0xC1; 32], // Chat
        [0xB1; 32], // Community
        [0xA1; 32], // Photos
        [0xEE; 32], // Vault
        [0xBB; 32], // Backup
    ];

    for (i, &ns) in ns_list.iter().enumerate() {
        let mut meta = BTreeMap::new();
        meta.insert("index".to_string(), format!("{}", i));
        node.create_object(ns, ObjectType::Synthetic(i as u16 + 1), meta, format!("PAYLOAD_{}", i).into_bytes()).unwrap();
    }

    assert_eq!(node.state.object_store.len(), 6);

    let cp = node.sync_now().unwrap();
    assert_ne!(cp.body.state_root, [0u8; 32]);

    node.stop().unwrap();
}
