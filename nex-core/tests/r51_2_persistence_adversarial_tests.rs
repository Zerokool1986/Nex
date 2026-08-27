use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::storage::wal::WriteAheadLog;

#[test]
fn test_r51_2_a_torn_wal_tail_autotruncation() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let wal_path = data_dir.join("wal.log");

    // 1. Create node and write 10 objects
    {
        let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
        node.start().unwrap();

        for i in 0..10 {
            let mut meta = BTreeMap::new();
            meta.insert("index".to_string(), format!("{}", i));
            node.create_object(
                [0x01; 32],
                ObjectType::Synthetic(1),
                meta,
                format!("PERSIST_PAYLOAD_{}", i).into_bytes(),
            ).unwrap();
        }

        node.stop().unwrap();
    }

    let initial_wal_len = fs::metadata(&wal_path).unwrap().len();

    // 2. Inject 17 trailing corrupt bytes (simulating power cut during partial write)
    {
        let mut f = OpenOptions::new().append(true).open(&wal_path).unwrap();
        f.write_all(b"CORRUPT_TAIL_GARB").unwrap();
        f.sync_all().unwrap();
    }

    let corrupted_wal_len = fs::metadata(&wal_path).unwrap().len();
    assert_eq!(corrupted_wal_len, initial_wal_len + 17);

    // 3. Recover WAL -> must recover all 10 mutations and auto-truncate the 17 corrupt bytes
    let recovered_mutations = WriteAheadLog::recover(&wal_path).expect("Recovery must succeed");
    assert_eq!(recovered_mutations.len(), 10);

    let truncated_wal_len = fs::metadata(&wal_path).unwrap().len();
    assert_eq!(truncated_wal_len, initial_wal_len, "WAL must be auto-truncated back to last valid record offset");
}

#[test]
fn test_r51_2_b_append_after_torn_wal_truncation() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let wal_path = data_dir.join("wal.log");

    // 1. Initial 5 objects
    {
        let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
        node.start().unwrap();
        for i in 0..5 {
            let mut meta = BTreeMap::new();
            meta.insert("i".to_string(), format!("{}", i));
            node.create_object([0x01; 32], ObjectType::Synthetic(1), meta, vec![i as u8; 32]).unwrap();
        }
        node.stop().unwrap();
    }

    // 2. Inject corrupt trailing bytes
    {
        let mut f = OpenOptions::new().append(true).open(&wal_path).unwrap();
        f.write_all(b"PARTIAL_TORN_RECORD_12345").unwrap();
        f.sync_all().unwrap();
    }

    // 3. Restart node -> auto-truncates and replays 5 objects
    {
        let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
        node.start().unwrap();
        assert_eq!(node.state.object_store.len(), 5);

        // 4. Append 5 more objects
        for i in 5..10 {
            let mut meta = BTreeMap::new();
            meta.insert("i".to_string(), format!("{}", i));
            node.create_object([0x01; 32], ObjectType::Synthetic(1), meta, vec![i as u8; 32]).unwrap();
        }
        assert_eq!(node.state.object_store.len(), 10);
        node.stop().unwrap();
    }

    // 5. Final recovery assertion: all 10 records are clean and valid
    let final_mutations = WriteAheadLog::recover(&wal_path).unwrap();
    assert_eq!(final_mutations.len(), 10);
}

#[test]
fn test_r51_2_c_parent_directory_fsync_durability_barrier() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(data_dir.clone(), signing_key);
    node.start().unwrap();

    for i in 0..10 {
        let mut meta = BTreeMap::new();
        meta.insert("k".to_string(), format!("{}", i));
        node.create_object([0x02; 32], ObjectType::Synthetic(1), meta, vec![0xBB; 64]).unwrap();
    }

    let checkpoint = node.checkpoint_and_compact().expect("Checkpoint with parent fsync must succeed");
    assert_ne!(checkpoint.body.state_root, [0u8; 32]);
    assert!(data_dir.join("state.db").exists());

    node.stop().unwrap();
}

#[test]
fn test_r51_2_d_stale_pid_lockfile_autorecovery() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    fs::create_dir_all(&data_dir).unwrap();

    // Manually write a stale lockfile with a dead PID (e.g. 99999999)
    let lock_path = data_dir.join(".nex.lock");
    fs::write(&lock_path, "99999999\n").unwrap();
    assert!(lock_path.exists());

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(data_dir.clone(), signing_key);

    // Node must detect that PID 99999999 is dead, auto-clean the lockfile, and start cleanly
    let start_res = node.start();
    assert!(start_res.is_ok(), "Node must recover from stale orphan lockfile");

    // Lockfile now contains current live process PID
    let new_lock_content = fs::read_to_string(&lock_path).unwrap();
    assert_eq!(new_lock_content.trim(), std::process::id().to_string());

    node.stop().unwrap();
    assert!(!lock_path.exists(), "Lockfile must be cleaned up on graceful stop");
}

#[test]
fn test_r51_2_e_active_daemon_lockfile_exclusivity() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let key1 = SigningKey::generate(&mut csprng);
    let key2 = SigningKey::generate(&mut csprng);

    let mut node1 = NexNode::new(data_dir.clone(), key1);
    node1.start().expect("Node 1 must start cleanly");

    // Attempt to start second node on same data_dir while Node 1 is alive
    let mut node2 = NexNode::new(data_dir.clone(), key2.clone());
    let res = node2.start();
    assert!(res.is_err(), "Second node instance must be rejected when active daemon PID holds lock");

    // Stop Node 1
    node1.stop().unwrap();

    // Now Node 2 can start cleanly
    let res2 = node2.start();
    assert!(res2.is_ok(), "Node 2 can acquire lock after Node 1 stops");
    node2.stop().unwrap();
}

#[test]
fn test_r51_2_f_zero_regression_across_durability_lifecycle() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut expected_total_objects = 0;

    for cycle in 0..5 {
        let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
        node.start().unwrap();

        for i in 0..4 {
            let mut meta = BTreeMap::new();
            meta.insert("cycle".to_string(), format!("{}", cycle));
            meta.insert("item".to_string(), format!("{}", i));
            node.create_object(
                [0x03; 32],
                ObjectType::Synthetic(1),
                meta,
                format!("CYCLE_{}_ITEM_{}", cycle, i).into_bytes(),
            ).unwrap();
            expected_total_objects += 1;
        }

        if cycle % 2 == 1 {
            node.checkpoint_and_compact().unwrap();
        }

        // Simulate crash: process abruptly terminated
        drop(node);

        // Simulated dead PID in lockfile + trailing partial bytes in WAL
        let lock_path = data_dir.join(".nex.lock");
        let _ = fs::write(&lock_path, "99999999\n");

        let wal_path = data_dir.join("wal.log");
        if wal_path.exists() {
            let mut f = OpenOptions::new().append(true).open(&wal_path).unwrap();
            let _ = f.write_all(b"TRN");
            let _ = f.sync_all();
        }
    }

    // Final reboot after 5 crash cycles
    let mut final_node = NexNode::new(data_dir.clone(), signing_key);
    final_node.start().expect("Final reboot must succeed after 5 crash cycles");
    assert_eq!(final_node.state.object_store.len(), expected_total_objects);

    let final_cp = final_node.checkpoint_and_compact().unwrap();
    assert_ne!(final_cp.body.state_root, [0u8; 32]);
    final_node.stop().unwrap();
}
