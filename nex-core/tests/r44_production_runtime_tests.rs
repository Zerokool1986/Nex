use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tempfile::tempdir;
use nex_core::runtime::production::{ProductionNodeSupervisor, NodeOperationalState};
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::identity::types::{KeyType, CapabilityProof, OP_READ, OP_WRITE};
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r44_a_node_lifecycle_lockfile_and_shutdown() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut supervisor = ProductionNodeSupervisor::new(&data_dir, key);
    assert_eq!(supervisor.state, NodeOperationalState::Uninitialized);

    // 1. Start daemon -> acquires lockfile
    supervisor.start().unwrap();
    assert_eq!(supervisor.state, NodeOperationalState::Running);
    assert!(data_dir.join(".nex.lock").exists());

    // 2. Second instance attempting to start on same data dir -> Must FAIL
    let key2 = SigningKey::generate(&mut csprng);
    let mut supervisor2 = ProductionNodeSupervisor::new(&data_dir, key2);
    assert!(supervisor2.start().is_err(), "Second daemon instance must be rejected by lockfile");

    // 3. Stop daemon -> releases lockfile cleanly
    supervisor.stop().unwrap();
    assert_eq!(supervisor.state, NodeOperationalState::Stopped);
    assert!(!data_dir.join(".nex.lock").exists());
}

#[test]
fn test_r44_b_cas_mark_and_sweep_garbage_collection() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut supervisor = ProductionNodeSupervisor::new(tmp.path(), key);

    // 1. Store 3 chunks
    let chunk1 = vec![0x11; 1024];
    let chunk2 = vec![0x22; 1024];
    let chunk3 = vec![0x33; 1024];

    let (root1, _) = supervisor.cas.store_file(&chunk1);
    let (root2, _) = supervisor.cas.store_file(&chunk2);
    let (root3, _) = supervisor.cas.store_file(&chunk3);

    assert_eq!(supervisor.cas.chunks.len(), 3);

    // 2. Live roots only contain chunk1 and chunk3 (chunk2 is unreferenced)
    let mut live_roots = HashSet::new();
    live_roots.insert(root1);
    live_roots.insert(root3);

    // 3. Run GC
    let reclaimed = supervisor.gc_cas_unreachable(&live_roots);
    assert_eq!(reclaimed, 1, "GC must reclaim exactly 1 unreferenced chunk");
    assert_eq!(supervisor.cas.chunks.len(), 2);
    assert!(supervisor.cas.has_chunk(&root1));
    assert!(!supervisor.cas.has_chunk(&root2));
    assert!(supervisor.cas.has_chunk(&root3));
}

#[test]
fn test_r44_f_structured_observability_json_events() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let supervisor = ProductionNodeSupervisor::new(tmp.path(), key);

    let mut details = BTreeMap::new();
    details.insert("peer_id".to_string(), "0x99aa".to_string());
    details.insert("bytes_synced".to_string(), "4096".to_string());

    let json_str = supervisor.emit_log_event("INFO", "sync", "peer_sync_completed", details);
    assert!(json_str.contains("\"subsystem\":\"sync\""));
    assert!(json_str.contains("\"event_type\":\"peer_sync_completed\""));
    assert!(json_str.contains("\"bytes_synced\":\"4096\""));
}

#[test]
fn test_r44_g_schema_migration_and_atomic_rollback() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut supervisor = ProductionNodeSupervisor::new(tmp.path(), key);
    assert_eq!(supervisor.schema_version, 1);

    // 1. Successful migration (v1 -> v2)
    supervisor.execute_schema_migration(2, false).unwrap();
    assert_eq!(supervisor.schema_version, 2);

    // 2. Failed migration (v2 -> v3) -> Must rollback to v2
    let res_fail = supervisor.execute_schema_migration(3, true);
    assert!(res_fail.is_err());
    assert_eq!(supervisor.schema_version, 2, "Failed migration must roll back to previous schema version");
}
