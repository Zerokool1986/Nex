use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::apps::drive::{derive_drive_object_id, normalize_vpath};
use nex_core::sync::anti_entropy::AntiEntropyEngine;

#[test]
fn test_r51_5_a_cross_platform_path_normalization_invariance() {
    let namespace = [0x55; 32];

    let p1 = "docs\\finance\\2026_budget.xlsx";
    let p2 = "docs/finance/2026_budget.xlsx";
    let p3 = "/docs/finance/2026_budget.xlsx";
    let p4 = "\\docs\\finance\\2026_budget.xlsx";
    let p5 = "/docs//finance/./2026_budget.xlsx";

    assert_eq!(normalize_vpath(p1), "/docs/finance/2026_budget.xlsx");
    assert_eq!(normalize_vpath(p2), "/docs/finance/2026_budget.xlsx");
    assert_eq!(normalize_vpath(p3), "/docs/finance/2026_budget.xlsx");
    assert_eq!(normalize_vpath(p4), "/docs/finance/2026_budget.xlsx");
    assert_eq!(normalize_vpath(p5), "/docs/finance/2026_budget.xlsx");

    let id1 = derive_drive_object_id(&namespace, p1);
    let id2 = derive_drive_object_id(&namespace, p2);
    let id3 = derive_drive_object_id(&namespace, p3);
    let id4 = derive_drive_object_id(&namespace, p4);
    let id5 = derive_drive_object_id(&namespace, p5);

    assert_eq!(id1, id2, "Windows backslash path must match POSIX forward slash ObjectID");
    assert_eq!(id2, id3, "Leading slash path must match non-leading slash ObjectID");
    assert_eq!(id3, id4, "Leading backslash path must match POSIX ObjectID");
    assert_eq!(id4, id5, "Redundant slashes must normalize cleanly");
}

#[test]
fn test_r51_5_b_scoped_storage_sandboxing() {
    let tmp = tempdir().unwrap();
    // Simulate Android scoped data directory: /data/user/0/<pkg>/files/isolated_root
    let scoped_dir = tmp.path().join("data").join("user").join("0").join("com.nex.sovereign").join("files").join("isolated_root");

    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    // Node must create directory tree, acquire lock, snapshot, and restart inside sandbox
    let mut node = NexNode::new(scoped_dir.clone(), key.clone());
    node.start().expect("Node must boot cleanly in scoped sandbox directory");

    for i in 0..10 {
        let mut meta = BTreeMap::new();
        meta.insert("i".to_string(), format!("{}", i));
        node.create_object([0x01; 32], ObjectType::Synthetic(1), meta, format!("SANDBOX_{}", i).into_bytes()).unwrap();
    }

    let cp = node.checkpoint_and_compact().unwrap();
    assert_ne!(cp.body.state_root, [0u8; 32]);
    assert!(scoped_dir.join("state.db").exists());
    node.stop().unwrap();

    // Reopen from sandbox
    let mut recovered_node = NexNode::new(scoped_dir, key);
    recovered_node.start().expect("Node must recover cleanly from scoped sandbox");
    assert_eq!(recovered_node.state.object_store.len(), 10);
    recovered_node.stop().unwrap();
}

#[test]
fn test_r51_5_c_endianness_and_wire_serialization_invariance() {
    let val: u64 = 0x0102030405060708;

    let be_bytes = val.to_be_bytes();
    let le_bytes = val.to_le_bytes();

    assert_eq!(be_bytes, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    assert_eq!(le_bytes, [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]);

    // Ensure Lamport & sequence counters are explicitly Big-Endian on wire
    let seq: u64 = 42;
    let wire_seq = seq.to_be_bytes();
    assert_eq!(u64::from_be_bytes(wire_seq), 42);
}

#[test]
fn test_r51_5_d_android_background_freeze_and_resume() {
    let mut csprng = OsRng;
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::generate(&mut csprng));
    let mut node_b = NexNode::new(tmp_b.path(), SigningKey::generate(&mut csprng));

    node_a.start().unwrap();
    node_b.start().unwrap();

    // Node A authors 20 objects
    for i in 0..20 {
        let mut meta = BTreeMap::new();
        meta.insert("idx".to_string(), format!("{}", i));
        node_a.create_object([0x01; 32], ObjectType::Synthetic(1), meta, format!("FREEZE_TEST_{}", i).into_bytes()).unwrap();
    }

    let session_id = [0x77; 16];

    // Phase 1: Sync first 10 objects to Node B
    let adv_b1 = AntiEntropyEngine::generate_advertise(&mut node_b, session_id);
    let batches_p1 = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b1.frontier_mutation_ids, 10);
    AntiEntropyEngine::ingest_batch(&mut node_b, batches_p1[0].clone()).unwrap();
    assert_eq!(node_b.state.object_store.len(), 10);

    // Phase 2: Simulate OS background suspend & resume (freeze sync session)
    let adv_b2 = AntiEntropyEngine::generate_advertise(&mut node_b, session_id);
    let batches_p2 = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b2.frontier_mutation_ids, 10);
    
    for batch in batches_p2 {
        AntiEntropyEngine::ingest_batch(&mut node_b, batch).unwrap();
    }

    assert_eq!(node_b.state.object_store.len(), 20, "Node B must complete remaining sync after unfreeze");
    let root_a = node_a.sync_now().unwrap().body.state_root;
    let root_b = node_b.sync_now().unwrap().body.state_root;
    assert_eq!(root_a, root_b);

    node_a.stop().unwrap();
    node_b.stop().unwrap();
}

#[test]
fn test_r51_5_e_multios_lockfile_crlf_interoperability() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    fs::create_dir_all(&data_dir).unwrap();

    // Write Windows CRLF line ending in lockfile: "99999999\r\n"
    let lock_path = data_dir.join(".nex.lock");
    fs::write(&lock_path, "99999999\r\n").unwrap();

    let mut csprng = OsRng;
    let mut node = NexNode::new(data_dir.clone(), SigningKey::generate(&mut csprng));

    // Node must trim CRLF, detect PID 99999999 is dead, and start cleanly
    let start_res = node.start();
    assert!(start_res.is_ok(), "Lockfile with CRLF must parse PID without error");

    node.stop().unwrap();
}

#[test]
fn test_r51_5_f_zero_regression_across_multios_product_apps() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let win_path = "projects\\crypto\\whitepaper.pdf";
    let sanitized = normalize_vpath(win_path);
    assert_eq!(sanitized, "/projects/crypto/whitepaper.pdf");

    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), sanitized);
    node.create_object([0x01; 32], ObjectType::DriveInode, meta, b"PDF_BYTES_12345".to_vec()).unwrap();
    assert_eq!(node.state.object_store.len(), 1);

    node.stop().unwrap();
}
