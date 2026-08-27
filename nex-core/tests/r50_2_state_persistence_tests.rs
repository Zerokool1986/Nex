use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;

#[test]
fn test_r50_2_a_atomic_two_phase_snapshotting() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // 1. Create node, write objects, and take checkpoint
    {
        let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
        node.start().unwrap();

        for i in 0..20 {
            let mut meta = BTreeMap::new();
            meta.insert("index".to_string(), format!("{}", i));
            node.create_object(
                [0xAA; 32],
                ObjectType::Synthetic(1),
                meta,
                format!("SNAPSHOT_PAYLOAD_{}", i).into_bytes(),
            ).unwrap();
        }

        let cp = node.checkpoint_and_compact().expect("Checkpoint and compact must succeed");
        assert_ne!(cp.body.state_root, [0u8; 32]);
        assert!(data_dir.join("state.db").exists(), "state.db must exist on disk");

        node.stop().unwrap();
    }

    // 2. Reboot from state.db
    {
        let mut recovered_node = NexNode::new(data_dir, signing_key);
        recovered_node.start().expect("Node must start cleanly from state.db snapshot");

        assert_eq!(recovered_node.state.object_store.len(), 20, "All 20 objects must be loaded from state.db");
        assert_eq!(recovered_node.state.state_node.crdt_state.len(), 20);

        recovered_node.stop().unwrap();
    }
}

#[test]
fn test_r50_2_b_torn_staging_file_defense() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // 1. Establish valid state.db
    let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
    node.start().unwrap();
    node.create_object([0xBB; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"VALID_DATA".to_vec()).unwrap();
    node.checkpoint_and_compact().unwrap();
    node.stop().unwrap();

    // 2. Simulate crash leaving corrupted state.db.tmp on disk
    let mut tmp_file = File::create(data_dir.join("state.db.tmp")).unwrap();
    tmp_file.write_all(b"CORRUPTED_INCOMPLETE_TORN_STAGING_BYTES").unwrap();
    tmp_file.flush().unwrap();

    // 3. Reboot: Node must ignore state.db.tmp and safely boot from state.db
    let mut recovered = NexNode::new(data_dir, signing_key);
    recovered.start().expect("Node must safely boot from valid state.db ignoring torn tmp file");
    assert_eq!(recovered.state.object_store.len(), 1);
    recovered.stop().unwrap();
}

#[test]
fn test_r50_2_c_wal_compaction_post_checkpoint() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
    node.start().unwrap();

    for i in 0..50 {
        node.create_object(
            [0xCC; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            format!("COMPACT_PAYLOAD_{}", i).into_bytes(),
        ).unwrap();
    }

    let pre_compact_size = fs::metadata(data_dir.join("wal.log")).unwrap().len();
    assert!(pre_compact_size > 2000, "Uncompacted WAL must be > 2000 bytes (got {})", pre_compact_size);

    // Checkpoint & Compact
    node.checkpoint_and_compact().unwrap();
    let post_compact_size = fs::metadata(data_dir.join("wal.log")).unwrap().len();
    assert_eq!(post_compact_size, 8, "Compacted WAL must be truncated to 8-byte header");

    // Write 5 new mutations to WAL tail
    for i in 0..5 {
        node.create_object(
            [0xCC; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            format!("TAIL_PAYLOAD_{}", i).into_bytes(),
        ).unwrap();
    }

    node.stop().unwrap();

    // Reboot: 50 from snapshot + 5 from WAL tail = 55 total objects
    let mut recovered = NexNode::new(data_dir, signing_key);
    recovered.start().unwrap();
    assert_eq!(recovered.state.object_store.len(), 55, "Node must reconstruct 50 snapshot + 5 WAL tail objects");
    recovered.stop().unwrap();
}

#[test]
fn test_r50_2_d_sub_50ms_recovery_sla() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
    node.start().unwrap();

    // Ingest 200 objects and checkpoint
    for i in 0..200 {
        node.create_object(
            [0xDD; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            format!("PERF_OBJ_{}", i).into_bytes(),
        ).unwrap();
    }
    node.checkpoint_and_compact().unwrap();

    // Ingest 20 tail mutations
    for i in 0..20 {
        node.create_object(
            [0xDD; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            format!("PERF_TAIL_OBJ_{}", i).into_bytes(),
        ).unwrap();
    }
    node.stop().unwrap();

    // Measure recovery time
    let start = Instant::now();
    let mut recovered = NexNode::new(data_dir, signing_key);
    recovered.start().unwrap();
    let recovery_dur = start.elapsed();

    assert!(recovery_dur.as_millis() < 50, "Recovery from snapshot + tail must take <50ms SLA (took {:?})", recovery_dur);
    assert_eq!(recovered.state.object_store.len(), 220);
    recovered.stop().unwrap();
}

#[test]
fn test_r50_2_e_sparse_merkle_tree_integrity() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
    node.start().unwrap();

    node.create_object([0xEE; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"MERKLE_TEST_1".to_vec()).unwrap();
    node.create_object([0xEE; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"MERKLE_TEST_2".to_vec()).unwrap();

    let pre_cp = node.checkpoint_and_compact().unwrap();
    node.stop().unwrap();

    let mut recovered = NexNode::new(data_dir, signing_key);
    recovered.start().unwrap();
    let post_cp = recovered.sync_now().unwrap();

    assert_eq!(pre_cp.body.state_root, post_cp.body.state_root, "StateCommitment Merkle state root must match exactly post-recovery");
    recovered.stop().unwrap();
}

#[test]
fn test_r50_2_f_zero_regression() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
    node.start().unwrap();

    let obj_id = node.create_object([0xFF; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"INITIAL".to_vec()).unwrap();
    node.checkpoint_and_compact().unwrap();

    // Mutate object after reload
    node.stop().unwrap();
    let mut reloaded = NexNode::new(data_dir, signing_key);
    reloaded.start().unwrap();

    reloaded.mutate_object(obj_id, None, Some(b"MUTATED_POST_SNAPSHOT".to_vec()), None).unwrap();
    let read_obj = reloaded.read_object(&obj_id).unwrap();
    assert_eq!(read_obj.payload_bytes, b"MUTATED_POST_SNAPSHOT");

    reloaded.delete_object(obj_id, None).unwrap();
    assert!(reloaded.read_object(&obj_id).is_err(), "Deleted object must return tombstoned error");

    reloaded.stop().unwrap();
}
