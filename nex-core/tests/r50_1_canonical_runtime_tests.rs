use std::collections::BTreeMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::production::ProductionNodeSupervisor;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;

#[test]
fn test_r50_1_a_clean_boot_and_pid_exclusivity() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // 1. First instance boots and acquires lock
    let mut node1 = NexNode::new(data_dir.clone(), signing_key.clone());
    node1.start().expect("Clean boot must succeed");
    assert!(data_dir.join(".nex.lock").exists(), "Exclusivity lockfile must exist on disk");

    // 2. Second instance on same data_dir must be rejected
    let mut node2 = NexNode::new(data_dir.clone(), signing_key);
    let start2_res = node2.start();
    assert!(start2_res.is_err(), "Second node instance must fail to acquire lockfile");

    // 3. Stop first instance cleanly
    node1.stop().expect("Clean stop must succeed");
    assert!(!data_dir.join(".nex.lock").exists(), "Lockfile must be unlinked on stop");
}

#[test]
fn test_r50_1_b_single_canonical_mutation_path() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(data_dir.clone(), signing_key);
    node.start().unwrap();

    let namespace = [0x50; 32];
    let mut meta = BTreeMap::new();
    meta.insert("name".to_string(), "unified_doc.pdf".to_string());
    let payload = b"UNIFIED_CANONICAL_MUTATION_PAYLOAD_V1".to_vec();

    // Ingest mutation via canonical NexAppApi
    let obj_id = node.create_object(namespace, ObjectType::Synthetic(1), meta, payload.clone())
        .expect("Object creation via canonical path must succeed");

    // Verify object read
    let obj = node.read_object(&obj_id).expect("Created object must be readable");
    assert_eq!(obj.payload_bytes, payload);
    assert_eq!(obj.namespace, namespace);

    // Verify DAG contains mutation
    assert!(node.state.latest_mutation_id.is_some());
    let m_id = node.state.latest_mutation_id.unwrap();
    assert!(node.state.state_node.dag.contains_key(&m_id));

    node.stop().unwrap();
}

#[test]
fn test_r50_1_c_concurrent_ingress_stream() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let node = Arc::new(Mutex::new(NexNode::new(data_dir, signing_key)));
    node.lock().unwrap().start().unwrap();

    let mut handles = Vec::new();

    // Spawn 5 concurrent worker threads ingesting mutations
    for t_idx in 0..5 {
        let node_ref = Arc::clone(&node);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                let namespace = [0x77; 32];
                let mut meta = BTreeMap::new();
                meta.insert("thread".to_string(), format!("{}", t_idx));
                meta.insert("iter".to_string(), format!("{}", i));
                let payload = format!("CONCURRENT_DATA_{}_{}", t_idx, i).into_bytes();

                let mut locked = node_ref.lock().unwrap();
                let res = locked.create_object(namespace, ObjectType::Synthetic(2), meta, payload);
                assert!(res.is_ok(), "Concurrent mutation must succeed");
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let mut locked_node = node.lock().unwrap();
    assert_eq!(locked_node.state.object_store.len(), 50, "All 50 concurrent objects must be present in StateEngine");
    assert_eq!(locked_node.state.state_node.dag.len(), 50, "DAG must contain all 50 mutations");

    locked_node.stop().unwrap();
}

#[test]
fn test_r50_1_d_graceful_shutdown_sla() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(data_dir.clone(), signing_key);
    node.start().unwrap();

    for i in 0..100 {
        node.create_object(
            [0x01; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            format!("BURST_OBJECT_{}", i).into_bytes(),
        ).unwrap();
    }

    let start = Instant::now();
    node.stop().expect("Stop must succeed");
    let shutdown_dur = start.elapsed();

    assert!(shutdown_dur.as_millis() < 500, "Graceful shutdown must complete in <500ms SLA (took {:?})", shutdown_dur);
    assert!(!data_dir.join(".nex.lock").exists(), "Lockfile must be unlinked");
}

#[test]
fn test_r50_1_e_ungraceful_crash_and_wal_replay() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // 1. Active mutations followed by sudden crash (no stop())
    {
        let mut node = NexNode::new(data_dir.clone(), signing_key.clone());
        node.start().unwrap();
        for i in 0..25 {
            node.create_object(
                [0x02; 32],
                ObjectType::Synthetic(1),
                BTreeMap::new(),
                format!("CRASH_OBJECT_{}", i).into_bytes(),
            ).unwrap();
        }
        // Simulated process termination
    }

    // 2. Recover from disk
    let _ = fs::remove_file(data_dir.join(".nex.lock"));
    let mut recovered = NexNode::new(data_dir.clone(), signing_key);
    recovered.start().expect("Recovered node must start cleanly from WAL");

    assert_eq!(recovered.state.state_node.dag.len(), 25, "All 25 mutations must be replayed from WAL into DAG");
    recovered.stop().unwrap();
}

#[test]
fn test_r50_1_f_backward_compatibility() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // Verify existing ProductionNodeSupervisor continues to function identically
    let mut supervisor = ProductionNodeSupervisor::new(data_dir.clone(), signing_key);
    supervisor.start().unwrap();
    assert_eq!(supervisor.schema_version, 1);
    supervisor.stop().unwrap();
}
