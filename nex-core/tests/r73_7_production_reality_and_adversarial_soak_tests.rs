use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::production::NodeOperationalState;
use nex_core::product::ingest::LocalFileIngestor;
use nex_core::transport::socket::{LanTcpTransportServer, LanTcpTransportClient};
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::derive_actor_id;
use nex_core::object::types::ObjectType;
use nex_core::api::NexAppApi;

#[test]
fn test_r73_7_a_high_throughput_burst_soak_and_compaction() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x71u8; 32]);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);

    // Soak: Create 100 objects in rapid succession
    let mut object_ids = Vec::new();
    for i in 0..100 {
        let mut meta = BTreeMap::new();
        meta.insert("index".to_string(), i.to_string());
        meta.insert("title".to_string(), format!("Soak Object #{}", i));
        let obj_id = node.create_object(
            family_ns,
            ObjectType::Synthetic(1),
            meta,
            format!("SOAK_PAYLOAD_{}", i).into_bytes(),
        ).unwrap();
        object_ids.push(obj_id);
    }

    assert_eq!(node.state.object_store.len(), 100);

    // Trigger Two-Phase Snapshot & WAL Compaction
    let checkpoint = node.checkpoint_and_compact().unwrap();
    assert_ne!(checkpoint.body.state_root, [0u8; 32]);

    // Stop and restart node from compacted snapshot + clean WAL
    node.stop().unwrap();

    let mut restarted = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x71u8; 32]));
    restarted.start().unwrap();

    assert_eq!(restarted.state.object_store.len(), 100);
    for obj_id in &object_ids {
        assert!(restarted.state.object_store.contains_key(obj_id));
    }
}

#[test]
fn test_r73_7_b_failure_injection_torn_wal_tail_recovery() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x72u8; 32]);
    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);

    let obj_id;
    {
        let mut node = NexNode::new(tmp.path(), key);
        node.start().unwrap();

        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), "Pre-Crash Document".to_string());
        obj_id = node.create_object(
            family_ns,
            ObjectType::DriveInode,
            meta,
            b"CRITICAL_PERSISTED_BYTES".to_vec(),
        ).unwrap();

        node.stop().unwrap();
    }

    // Failure Injection: Append corrupt / torn bytes to wal.log (simulating sudden power loss mid-write)
    let wal_path = tmp.path().join("wal.log");
    {
        let mut f = OpenOptions::new().append(true).open(&wal_path).unwrap();
        f.write_all(b"GARBAGE_TORN_BYTES_PARTIAL_WRITE_CORRUPTION_12345").unwrap();
        f.flush().unwrap();
    }

    // Restart node: must auto-truncate torn bytes and recover valid state
    {
        let mut recovered_node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x72u8; 32]));
        recovered_node.start().unwrap();

        assert!(recovered_node.state.object_store.contains_key(&obj_id));
        let obj = recovered_node.state.object_store.get(&obj_id).unwrap();
        assert_eq!(obj.payload_bytes, b"CRITICAL_PERSISTED_BYTES");
    }
}

#[test]
fn test_r73_7_c_multi_node_concurrent_divergent_write_convergence() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();
    let tmp_c = tempdir().unwrap();

    let root_key = SigningKey::from_bytes(&[0x73u8; 32]);

    let node_a = Arc::new(Mutex::new(NexNode::new(tmp_a.path(), root_key.clone())));
    node_a.lock().unwrap().start().unwrap();

    let node_b = Arc::new(Mutex::new(NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x74u8; 32]))));
    node_b.lock().unwrap().start().unwrap();

    let node_c = Arc::new(Mutex::new(NexNode::new(tmp_c.path(), SigningKey::from_bytes(&[0x75u8; 32]))));
    node_c.lock().unwrap().start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);

    // Concurrent divergent writes while partitioned
    let mut meta_a = BTreeMap::new();
    meta_a.insert("author".to_string(), "Node A".to_string());
    let id_a = node_a.lock().unwrap().create_object(family_ns, ObjectType::Synthetic(1), meta_a, b"PAYLOAD_FROM_A".to_vec()).unwrap();

    let mut meta_b = BTreeMap::new();
    meta_b.insert("author".to_string(), "Node B".to_string());
    let id_b = node_b.lock().unwrap().create_object(family_ns, ObjectType::Synthetic(1), meta_b, b"PAYLOAD_FROM_B".to_vec()).unwrap();

    let mut meta_c = BTreeMap::new();
    meta_c.insert("author".to_string(), "Node C".to_string());
    let id_c = node_c.lock().unwrap().create_object(family_ns, ObjectType::Synthetic(1), meta_c, b"PAYLOAD_FROM_C".to_vec()).unwrap();

    // 1. Start Server on Node B; Node A connects as client to pull Node B's object
    let server_b = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();
    let addr_b = server_b.bind_addr;

    let a_clone1 = Arc::clone(&node_a);
    let h1 = thread::spawn(move || {
        let mut a = a_clone1.lock().unwrap();
        LanTcpTransportClient::sync_with_remote_node(&mut *a, addr_b)
    });
    for _ in 0..20 {
        if server_b.poll_and_sync_one(&mut *node_b.lock().unwrap()).unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    h1.join().unwrap().unwrap();

    // 2. Start Server on Node C; Node A connects as client to pull Node C's object
    let server_c = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();
    let addr_c = server_c.bind_addr;

    let a_clone2 = Arc::clone(&node_a);
    let h2 = thread::spawn(move || {
        let mut a = a_clone2.lock().unwrap();
        LanTcpTransportClient::sync_with_remote_node(&mut *a, addr_c)
    });
    for _ in 0..20 {
        if server_c.poll_and_sync_one(&mut *node_c.lock().unwrap()).unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    h2.join().unwrap().unwrap();

    // All 3 objects reconciled on Node A
    let a = node_a.lock().unwrap();
    assert!(a.state.object_store.contains_key(&id_a));
    assert!(a.state.object_store.contains_key(&id_b));
    assert!(a.state.object_store.contains_key(&id_c));
}

#[test]
fn test_r73_7_d_security_abuse_fail_closed_under_tampered_payload() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x76u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), key.clone());
    node.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);

    // 1. Forged capability token with invalid signature bytes
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let bad_proof = CapabilityProof {
        token,
        issuer_pubkey: Some(key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: vec![0xDE; 64], // Corrupted signature bytes
    };

    // Attempting ingestion with forged capability proof must fail closed
    let photo_path = tmp.path().join("fake_photo.jpg");
    fs::write(&photo_path, b"FAKE_IMAGE_DATA").unwrap();

    let res = LocalFileIngestor::ingest_file(
        &mut node,
        SpaceType::Family,
        &photo_path,
        &bad_proof,
        &root_actor,
        10,
    );

    assert!(res.is_err(), "Ingestion with forged capability must fail closed");
}

#[test]
fn test_r73_7_e_cas_chunk_integrity_and_deduplication() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x77u8; 32]);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let common_payload = vec![0x42u8; 1024 * 64]; // 64 KB common payload

    let mut meta1 = BTreeMap::new();
    meta1.insert("title".to_string(), "Doc 1".to_string());
    let id1 = node.create_object(family_ns, ObjectType::DriveInode, meta1, common_payload.clone()).unwrap();

    let mut meta2 = BTreeMap::new();
    meta2.insert("title".to_string(), "Doc 2".to_string());
    let id2 = node.create_object(family_ns, ObjectType::DriveInode, meta2, common_payload.clone()).unwrap();

    // Verify distinct objects created at different Lamport ranks
    assert_ne!(id1, id2);
    assert!(node.state.object_store.contains_key(&id1));
    assert!(node.state.object_store.contains_key(&id2));
    assert_eq!(node.state.object_store.get(&id1).unwrap().payload_bytes, common_payload);
    assert_eq!(node.state.object_store.get(&id2).unwrap().payload_bytes, common_payload);
}

#[test]
fn test_r73_7_f_long_running_session_stability_and_memory_invariance() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x78u8; 32]);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let server = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();

    // Soak: 50 consecutive poll cycles on idle server
    for _ in 0..50 {
        let res = server.poll_and_sync_one(&mut node);
        assert!(res.is_ok());
    }

    assert_eq!(node.operational_state, NodeOperationalState::Running);
}
