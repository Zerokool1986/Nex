use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::sync::anti_entropy::{AntiEntropyEngine, SyncStreamBatch};

#[test]
fn test_r50_4_a_2node_frontier_advertisement_and_delta_discovery() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut node_a = NexNode::new(tmp_a.path(), key_a);
    let mut node_b = NexNode::new(tmp_b.path(), key_b);
    node_a.start().unwrap();
    node_b.start().unwrap();

    // Node A creates 10 objects
    for i in 0..10 {
        node_a.create_object(
            [0x11; 32],
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            format!("PAYLOAD_A_{}", i).into_bytes(),
        ).unwrap();
    }

    let session_id = [0x55; 16];

    // 1. Node B advertises empty frontier
    let adv_b = AntiEntropyEngine::generate_advertise(&mut node_b, session_id);
    assert_eq!(adv_b.known_mutation_count, 0);

    // 2. Node A checks if it has deltas for Node B
    assert!(AntiEntropyEngine::has_deltas_for_peer(&node_a, &adv_b));

    // 3. Node A generates stream batches for Node B
    let batches = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b.frontier_mutation_ids, 64);
    assert!(!batches.is_empty(), "Batches must not be empty");

    // 4. Node B ingests batches
    for batch in batches {
        let ack = AntiEntropyEngine::ingest_batch(&mut node_b, batch)
            .expect("Node B must ingest batch cleanly");
        assert!(ack.ingested_count > 0);
    }

    assert_eq!(node_b.state.state_node.dag.len(), 10, "Node B must have all 10 mutations");
    assert_eq!(node_b.state.object_store.len(), 10, "Node B must have all 10 objects in store");

    node_a.stop().unwrap();
    node_b.stop().unwrap();
}

#[test]
fn test_r50_4_b_bidirectional_concurrent_convergence() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut node_a = NexNode::new(tmp_a.path(), key_a);
    let mut node_b = NexNode::new(tmp_b.path(), key_b);
    node_a.start().unwrap();
    node_b.start().unwrap();

    // Node A writes 5 objects
    for i in 0..5 {
        node_a.create_object([0x22; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!("A_{}", i).into_bytes()).unwrap();
    }
    // Node B writes 5 objects
    for i in 0..5 {
        node_b.create_object([0x33; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!("B_{}", i).into_bytes()).unwrap();
    }

    let session_id = [0x77; 16];

    let adv_a = AntiEntropyEngine::generate_advertise(&mut node_a, session_id);
    let adv_b = AntiEntropyEngine::generate_advertise(&mut node_b, session_id);

    // Sync A -> B
    if AntiEntropyEngine::has_deltas_for_peer(&node_a, &adv_b) {\n        let batches_for_b = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b.frontier_mutation_ids, 64);\n        for b in batches_for_b {\n            AntiEntropyEngine::ingest_batch(&mut node_b, b).unwrap();\n        }\n    }\n\n    // Sync B -> A\n    if AntiEntropyEngine::has_deltas_for_peer(&node_b, &adv_a) {\n        let batches_for_a = AntiEntropyEngine::generate_batches_for_peer(&node_b, session_id, &adv_a.frontier_mutation_ids, 64);\n        for b in batches_for_a {\n            AntiEntropyEngine::ingest_batch(&mut node_a, b).unwrap();\n        }\n    }\n\n    assert_eq!(node_a.state.state_node.dag.len(), 10);\n    assert_eq!(node_b.state.state_node.dag.len(), 10);\n\n    // Mutual Convergence Verification\n    let comp_a = AntiEntropyEngine::generate_complete(&mut node_a, session_id);\n    let comp_b = AntiEntropyEngine::generate_complete(&mut node_b, session_id);\n\n    assert_eq!(comp_a.state_commitment, comp_b.state_commitment, \"StateCommitment roots must match exactly on convergence\");\n    assert!(AntiEntropyEngine::verify_convergence(&mut node_b, &comp_a).is_ok());\n\n    node_a.stop().unwrap();\n    node_b.stop().unwrap();\n}\n\n#[test]\nfn test_r50_4_c_sliding_window_credit_and_backpressure() {\n    let tmp = tempdir().unwrap();\n    let mut csprng = OsRng;\n    let key = SigningKey::generate(&mut csprng);\n\n    let mut node = NexNode::new(tmp.path(), key);\n    node.start().unwrap();\n\n    for i in 0..100 {\n        node.create_object([0x44; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!(\"BURST_{}\", i).into_bytes()).unwrap();\n    }\n\n    let req = nex_core::sync::anti_entropy::SyncDeltaRequest {\n        session_id: [0x11; 16],\n        requested_mutations: vec![],\n        max_batch_items: 25,\n    };\n\n    let batches = AntiEntropyEngine::generate_batches(&node, &req, &[]);\n    assert_eq!(batches.len(), 4, \"100 mutations in 25-item chunks must yield 4 batches\");\n\n    node.stop().unwrap();\n}\n\n#[test]\nfn test_r50_4_d_out_of_order_batch_reassembly() {\n    let tmp_src = tempdir().unwrap();\n    let tmp_dst = tempdir().unwrap();\n    let mut csprng = OsRng;\n    let key_src = SigningKey::generate(&mut csprng);\n    let key_dst = SigningKey::generate(&mut csprng);\n\n    let mut node_src = NexNode::new(tmp_src.path(), key_src);\n    let mut node_dst = NexNode::new(tmp_dst.path(), key_dst);\n    node_src.start().unwrap();\n    node_dst.start().unwrap();\n\n    for i in 0..10 {\n        node_src.create_object([0x55; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!(\"ORDER_{}\", i).into_bytes()).unwrap();\n    }\n\n    let req = nex_core::sync::anti_entropy::SyncDeltaRequest {\n        session_id: [0x22; 16],\n        requested_mutations: vec![],\n        max_batch_items: 5,\n    };\n    let batches = AntiEntropyEngine::generate_batches(&node_src, &req, &[]);\n    assert_eq!(batches.len(), 2);\n\n    // Ingest Batch 1 FIRST (out of order), then Batch 0\n    AntiEntropyEngine::ingest_batch(&mut node_dst, batches[1].clone()).unwrap();\n    AntiEntropyEngine::ingest_batch(&mut node_dst, batches[0].clone()).unwrap();\n\n    assert_eq!(node_dst.state.state_node.dag.len(), 10, \"All 10 items must be reassembled despite out-of-order ingress\");\n\n    node_src.stop().unwrap();\n    node_dst.stop().unwrap();\n}\n\n#[test]\nfn test_r50_4_e_malicious_forged_mutation_rejection() {\n    let tmp = tempdir().unwrap();\n    let mut csprng = OsRng;\n    let key = SigningKey::generate(&mut csprng);\n\n    let mut node = NexNode::new(tmp.path(), key);\n    node.start().unwrap();\n\n    node.create_object([0x66; 32], ObjectType::Synthetic(1), BTreeMap::new(), b\"AUTHENTIC\".to_vec()).unwrap();\n    let m = node.state.state_node.dag.values().next().unwrap().clone();\n\n    // Forge mutation ID\n    let mut forged_mutation = m.clone();\n    forged_mutation.id = [0x99; 32]; // ID does not match hash(body)\n\n    let bad_batch = SyncStreamBatch {\n        session_id: [0x33; 16],\n        batch_index: 0,\n        total_batches: 1,\n        mutations: vec![forged_mutation],\n    };\n\n    let res = AntiEntropyEngine::ingest_batch(&mut node, bad_batch);\n    assert!(res.is_err(), \"Forged mutation in batch must be rejected with preflight verification error\");\n\n    node.stop().unwrap();\n}\n\n#[test]\nfn test_r50_4_f_resume_interrupted_synchronization() {\n    let tmp_src = tempdir().unwrap();\n    let tmp_dst = tempdir().unwrap();\n    let mut csprng = OsRng;\n    let key_src = SigningKey::generate(&mut csprng);\n    let key_dst = SigningKey::generate(&mut csprng);\n\n    let mut node_src = NexNode::new(tmp_src.path(), key_src);\n    let mut node_dst = NexNode::new(tmp_dst.path(), key_dst);\n    node_src.start().unwrap();\n    node_dst.start().unwrap();\n\n    for i in 0..20 {\n        node_src.create_object([0x77; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!(\"RESUME_{}\", i).into_bytes()).unwrap();\n    }\n\n    let session1 = [0x44; 16];\n    let req1 = nex_core::sync::anti_entropy::SyncDeltaRequest {\n        session_id: session1,\n        requested_mutations: vec![],\n        max_batch_items: 10,\n    };\n    let batches = AntiEntropyEngine::generate_batches(&node_src, &req1, &[]);\n\n    // 1. Deliver Batch 0 only\n    AntiEntropyEngine::ingest_batch(&mut node_dst, batches[0].clone()).unwrap();\n    assert_eq!(node_dst.state.state_node.dag.len(), 10);\n\n    // 2. Disconnect & Reconnect (Session 2)\n    let session2 = [0x45; 16];\n    let adv_dst2 = AntiEntropyEngine::generate_advertise(&mut node_dst, session2);\n    assert!(AntiEntropyEngine::has_deltas_for_peer(&node_src, &adv_dst2));\n\n    let resume_batches = AntiEntropyEngine::generate_batches_for_peer(&node_src, session2, &adv_dst2.frontier_mutation_ids, 10);\n    assert_eq!(resume_batches.len(), 1, \"Resume must only generate 1 remaining batch for the 10 unsynced mutations\");\n\n    AntiEntropyEngine::ingest_batch(&mut node_dst, resume_batches[0].clone()).unwrap();\n    assert_eq!(node_dst.state.state_node.dag.len(), 20, \"Total 20 mutations after resume\");\n\n    node_src.stop().unwrap();\n    node_dst.stop().unwrap();\n}\n\n#[test]\nfn test_r50_4_g_out_of_order_mutation_crdt_lww_object_store_consistency() {\n    use nex_core::model::{Mutation, MutationBody, CrdtPayload};\n    use nex_core::hash::hash_mutation_body;\n\n    let tmp = tempdir().unwrap();\n    let mut csprng = OsRng;\n    let key = SigningKey::generate(&mut csprng);\n\n    let mut node = NexNode::new(tmp.path(), key);\n    node.start().unwrap();\n\n    let target_obj_id = [0xAA; 32];\n    let other_obj_id = [0xBB; 32];\n\n    // Genesis mutation for common causal ancestor (Lamport 0, Epoch 0)\n    let body_genesis = MutationBody {\n        author: [0x01; 32],\n        parents: vec![],\n        lamport: 0,\n        epoch: 0,\n        is_resurrect: false,\n        payload: CrdtPayload::AddLWW {\n            id: target_obj_id,\n            value: b\"GENESIS_VALUE\".to_vec(),\n        },\n    };\n    let m_genesis = Mutation::new(hash_mutation_body(&body_genesis), body_genesis);\n\n    // Branch A (Causally Earlier branch): Lamport 1, Epoch 0\n    let body_earlier = MutationBody {\n        author: [0x01; 32],\n        parents: vec![m_genesis.id],\n        lamport: 1,\n        epoch: 0,\n        is_resurrect: false,\n        payload: CrdtPayload::AddLWW {\n            id: target_obj_id,\n            value: b\"OBSOLETE_EARLIER_VALUE\".to_vec(),\n        },\n    };\n    let m_earlier = Mutation::new(hash_mutation_body(&body_earlier), body_earlier);\n\n    // Branch B (Causally Later branch):\n    // 1. Intermediate mutation (Lamport 1)\n    let body_inter = MutationBody {\n        author: [0x02; 32],\n        parents: vec![m_genesis.id],\n        lamport: 1,\n        epoch: 0,\n        is_resurrect: false,\n        payload: CrdtPayload::AddLWW {\n            id: other_obj_id,\n            value: b\"INTERMEDIATE\".to_vec(),\n        },\n    };\n    let m_inter = Mutation::new(hash_mutation_body(&body_inter), body_inter);\n\n    // 2. Later winning mutation (Lamport 2) on target_obj_id\n    let body_later = MutationBody {\n        author: [0x02; 32],\n        parents: vec![m_inter.id],\n        lamport: 2,\n        epoch: 0,\n        is_resurrect: false,\n        payload: CrdtPayload::AddLWW {\n            id: target_obj_id,\n            value: b\"WINNING_LATER_VALUE\".to_vec(),\n        },\n    };\n    let m_later = Mutation::new(hash_mutation_body(&body_later), body_later);\n\n    // Step 1: Ingest Genesis\n    let batch_genesis = SyncStreamBatch {\n        session_id: [0x99; 16],\n        batch_index: 0,\n        total_batches: 3,\n        mutations: vec![m_genesis],\n    };\n    AntiEntropyEngine::ingest_batch(&mut node, batch_genesis).unwrap();\n\n    // Step 2: Ingest Causally Later branch (m_inter, m_later) FIRST\n    let batch_later = SyncStreamBatch {\n        session_id: [0x99; 16],\n        batch_index: 1,\n        total_batches: 3,\n        mutations: vec![m_inter, m_later.clone()],\n    };\n    AntiEntropyEngine::ingest_batch(&mut node, batch_later).unwrap();\n\n    // Verify crdt_state and object_store currently hold WINNING_LATER_VALUE\n    let crdt_entry_init = node.state.state_node.crdt_state.get(&target_obj_id).unwrap();\n    assert_eq!(crdt_entry_init.3, m_later.id);\n    let obj_init = node.state.object_store.get(&target_obj_id).expect(\"Object must exist in object_store\");\n    assert_eq!(obj_init.payload_bytes, b\"WINNING_LATER_VALUE\".to_vec());\n\n    // Step 3: Ingest Causally Earlier mutation (m_earlier) SECOND (out of causal order)\n    let batch_earlier = SyncStreamBatch {\n        session_id: [0x99; 16],\n        batch_index: 2,\n        total_batches: 3,\n        mutations: vec![m_earlier],\n    };\n    AntiEntropyEngine::ingest_batch(&mut node, batch_earlier).unwrap();\n\n    // In crdt_state, m_later MUST WIN because Lamport 2 > Lamport 1\n    let crdt_entry = node.state.state_node.crdt_state.get(&target_obj_id).unwrap();\n    assert_eq!(crdt_entry.3, m_later.id, \"crdt_state must retain m_later as winner\");\n\n    // REGRESSION ASSERTION:\n    // object_store MUST ALSO retain WINNING_LATER_VALUE, NOT get overwritten by obsolete m_earlier!\n    let obj_final = node.state.object_store.get(&target_obj_id).expect(\"Object must exist in object_store\");\n    assert_eq!(\n        obj_final.payload_bytes,\n        b\"WINNING_LATER_VALUE\".to_vec(),\n        \"object_store must reflect the causal LWW winner regardless of batch arrival order\"\n    );\n\n    node.stop().unwrap();\n}\n