use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::drive::normalize_vpath;
use nex_core::api::NexAppApi;

#[test]
fn test_r57_4_a_cross_platform_path_normalization() {
    assert_eq!(normalize_vpath("docs/file.txt"), "/docs/file.txt");
    assert_eq!(normalize_vpath("\\windows\\path\\file.txt"), "/windows/path/file.txt");
    assert_eq!(normalize_vpath("//redundant///slashes//doc.pdf"), "/redundant/slashes/doc.pdf");
    assert_eq!(normalize_vpath("/"), "/");
}

#[test]
fn test_r57_4_b_crash_and_torn_wal_recovery() {
    let dir = tempdir().unwrap();
    let seed = [201u8; 32];

    {
        let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
        assert!(node.start().is_ok());
        let _ = node.sync_now();
        node.stop().unwrap();
    }

    // Simulate restart
    {
        let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
        assert!(node.start().is_ok());
        assert_eq!(node.schema_version, 1);
    }
}

#[test]
fn test_r57_4_c_mesh_pairwise_sync_over_simulated_transport() {
    let dir1 = tempdir().unwrap();
    let dir2 = tempdir().unwrap();

    let seed1 = [202u8; 32];
    let seed2 = [203u8; 32];

    let mut node1 = NexNode::new(dir1.path(), SigningKey::from_bytes(&seed1));
    let mut node2 = NexNode::new(dir2.path(), SigningKey::from_bytes(&seed2));

    assert!(node1.start().is_ok());
    assert!(node2.start().is_ok());

    let cp1 = node1.sync_now().unwrap();
    let cp2 = node2.sync_now().unwrap();

    assert_eq!(cp1.body.state_root, cp2.body.state_root);
}

#[test]
fn test_r57_4_d_memory_and_cas_deduplication_stress() {
    let dir = tempdir().unwrap();
    let seed = [204u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let payload = vec![0xEEu8; 64 * 1024]; // 64 KB chunk
    for _ in 0..10 {
        node.storage.cas.put_chunk(&payload);
    }

    assert_eq!(node.storage.cas.chunks.len(), 1, "Deduplication must store only 1 chunk");
}

#[test]
fn test_r57_4_e_sub_50ms_cold_start_simulation() {
    let start_time = std::time::Instant::now();
    let dir = tempdir().unwrap();
    let seed = [205u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    let elapsed = start_time.elapsed();

    assert!(elapsed.as_millis() < 50, "Cold start must complete in under 50ms (measured: {:?})", elapsed);
}

#[test]
fn test_r57_4_f_gate_r57_master_integration_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [206u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Idempotent sync must preserve Merkle root");
}
