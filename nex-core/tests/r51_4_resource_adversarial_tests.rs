use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::{NexAppApi, CoreRuntimeError};
use nex_core::object::types::ObjectType;
use nex_core::sync::anti_entropy::AntiEntropyEngine;
use nex_core::transport::session::{SessionKeys, NaspSessionManager};
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::sync::anti_entropy::SyncStreamBatch;

#[test]
fn test_r51_4_a_hard_2mb_payload_ceiling_preflight_shield() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    // 2.5 MB oversized payload
    let oversized_payload = vec![0xEE; (2.5 * 1024.0 * 1024.0) as usize];

    let res = node.create_object(
        [0x01; 32],
        ObjectType::Synthetic(1),
        BTreeMap::new(),
        oversized_payload.clone(),
    );

    match res {
        Err(CoreRuntimeError::InvalidPayload(msg)) => {
            assert!(msg.contains("exceeds limit"), "Error must clearly describe size limit: {}", msg);
        }
        other => panic!("Expected InvalidPayload error, got: {:?}", other),
    }

    assert_eq!(node.state.object_store.len(), 0, "No object must be stored on oversized payload");
    node.stop().unwrap();
}

#[test]
fn test_r51_4_b_hard_64kb_metadata_ceiling_shield() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    // 70 KB oversized metadata
    let mut oversized_metadata = BTreeMap::new();
    for i in 0..70 {
        oversized_metadata.insert(format!("key_{}", i), "X".repeat(1000));
    }

    let res = node.create_object(
        [0x01; 32],
        ObjectType::Synthetic(1),
        oversized_metadata,
        b"VALID_PAYLOAD".to_vec(),
    );

    match res {
        Err(CoreRuntimeError::InvalidPayload(msg)) => {
            assert!(msg.contains("exceeds limit"), "Error must describe metadata limit: {}", msg);
        }
        other => panic!("Expected InvalidPayload error, got: {:?}", other),
    }

    assert_eq!(node.state.object_store.len(), 0);
    node.stop().unwrap();
}

#[test]
fn test_r51_4_c_stack_safe_deep_dag_traversal() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    // Ingest 2,000 linear mutations into DAG
    let mut prev_id = [0u8; 32];
    for i in 0..2000u64 {
        let parents = if i == 0 { vec![] } else { vec![prev_id] };
        let body = MutationBody {
            author: [0x11; 32],
            parents,
            lamport: i,
            epoch: 0,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [i as u8; 32], value: vec![0xAA; 16] },
        };
        let m_id = node.execute_mutation(body).unwrap();
        prev_id = m_id;
    }

    assert_eq!(node.state.state_node.dag.len(), 2000);

    // Generate batches for peer with empty frontier -> traverses entire 2,000-deep DAG
    let session_id = [0x55; 16];
    let batches = AntiEntropyEngine::generate_batches_for_peer(&node, session_id, &[], 100);
    assert_eq!(batches.len(), 20); // 2,000 mutations / 100 per batch = 20 batches

    let total_synced_mutations: usize = batches.iter().map(|b| b.mutations.len()).sum();
    assert_eq!(total_synced_mutations, 2000, "All 2,000 mutations must be discovered iteratively");

    node.stop().unwrap();
}

#[test]
fn test_r51_4_d_sybil_session_bounding_and_lru_eviction() {
    let mut mgr = NaspSessionManager::new(256);

    // Insert 300 sessions
    for i in 0..300u16 {
        let mut session_id = [0u8; 16];
        session_id[0..2].copy_from_slice(&i.to_be_bytes());

        let dummy_keys = SessionKeys {
            is_initiator: true,
            k_tx: [i as u8; 32],
            k_rx: [i as u8; 32],
            k_mac_tx: [i as u8; 32],
            k_mac_rx: [i as u8; 32],
            k_rekey: [i as u8; 32],
            previous_k_rx: None,
            previous_k_mac_rx: None,
            tx_seq: 0,
            rx_seq: 0,
        };

        mgr.insert_session(session_id, dummy_keys);
    }

    assert_eq!(mgr.active_session_count(), 256, "Active sessions must be strictly capped at 256");

    // Check oldest session 0 was evicted
    let mut s0_id = [0u8; 16];
    s0_id[0..2].copy_from_slice(&0u16.to_be_bytes());
    assert!(mgr.get_session_mut(&s0_id).is_none(), "Oldest session must be evicted");

    // Check latest session 299 is active
    let mut s299_id = [0u8; 16];
    s299_id[0..2].copy_from_slice(&299u16.to_be_bytes());
    assert!(mgr.get_session_mut(&s299_id).is_some(), "Recent session must be present");
}

#[test]
fn test_r51_4_e_oversized_batch_and_malicious_mutation_shield() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let malicious_mutation = Mutation {
        id: [0xFF; 32], // Forged ID that does not match body hash
        body: MutationBody {
            author: [0x11; 32],
            parents: vec![],
            lamport: 0,
            epoch: 0,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [0xAA; 32], value: vec![0x11; 32] },
        },
    };

    let bad_batch = SyncStreamBatch {
        session_id: [0x99; 16],
        batch_index: 0,
        total_batches: 1,
        mutations: vec![malicious_mutation],
    };

    let res = AntiEntropyEngine::ingest_batch(&mut node, bad_batch);
    assert!(res.is_err(), "Forged mutation inside batch must be rejected by preflight verification");

    node.stop().unwrap();
}

#[test]
fn test_r51_4_f_zero_regression_under_sustained_resource_pressure() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    // Alternate between valid objects and rejected oversized attempts
    for i in 0..50 {
        // Valid 10KB object
        node.create_object(
            [0x01; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            vec![i as u8; 10 * 1024],
        ).unwrap();

        // Malicious 3MB payload attempt -> rejected
        let bad_res = node.create_object(
            [0x01; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            vec![0xFF; 3 * 1024 * 1024],
        );
        assert!(bad_res.is_err());
    }

    assert_eq!(node.state.object_store.len(), 50);
    let cp = node.checkpoint_and_compact().unwrap();
    assert_ne!(cp.body.state_root, [0u8; 32]);

    node.stop().unwrap();
}
