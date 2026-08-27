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
    if AntiEntropyEngine::has_deltas_for_peer(&node_a, &adv_b) {
        let batches_for_b = AntiEntropyEngine::generate_batches_for_peer(&node_a, session_id, &adv_b.frontier_mutation_ids, 64);
        for b in batches_for_b {
            AntiEntropyEngine::ingest_batch(&mut node_b, b).unwrap();
        }
    }

    // Sync B -> A
    if AntiEntropyEngine::has_deltas_for_peer(&node_b, &adv_a) {
        let batches_for_a = AntiEntropyEngine::generate_batches_for_peer(&node_b, session_id, &adv_a.frontier_mutation_ids, 64);
        for b in batches_for_a {
            AntiEntropyEngine::ingest_batch(&mut node_a, b).unwrap();
        }
    }

    assert_eq!(node_a.state.state_node.dag.len(), 10);
    assert_eq!(node_b.state.state_node.dag.len(), 10);

    // Mutual Convergence Verification
    let comp_a = AntiEntropyEngine::generate_complete(&mut node_a, session_id);
    let comp_b = AntiEntropyEngine::generate_complete(&mut node_b, session_id);

    assert_eq!(comp_a.state_commitment, comp_b.state_commitment, "StateCommitment roots must match exactly on convergence");
    assert!(AntiEntropyEngine::verify_convergence(&mut node_b, &comp_a).is_ok());

    node_a.stop().unwrap();
    node_b.stop().unwrap();
}

#[test]
fn test_r50_4_c_sliding_window_credit_and_backpressure() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    for i in 0..100 {
        node.create_object([0x44; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!("BURST_{}", i).into_bytes()).unwrap();
    }

    let req = nex_core::sync::anti_entropy::SyncDeltaRequest {
        session_id: [0x11; 16],
        requested_mutations: vec![],
        max_batch_items: 25,
    };

    let batches = AntiEntropyEngine::generate_batches(&node, &req, &[]);
    assert_eq!(batches.len(), 4, "100 mutations in 25-item chunks must yield 4 batches");

    node.stop().unwrap();
}

#[test]
fn test_r50_4_d_out_of_order_batch_reassembly() {
    let tmp_src = tempdir().unwrap();
    let tmp_dst = tempdir().unwrap();
    let mut csprng = OsRng;
    let key_src = SigningKey::generate(&mut csprng);
    let key_dst = SigningKey::generate(&mut csprng);

    let mut node_src = NexNode::new(tmp_src.path(), key_src);
    let mut node_dst = NexNode::new(tmp_dst.path(), key_dst);
    node_src.start().unwrap();
    node_dst.start().unwrap();

    for i in 0..10 {
        node_src.create_object([0x55; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!("ORDER_{}", i).into_bytes()).unwrap();
    }

    let req = nex_core::sync::anti_entropy::SyncDeltaRequest {
        session_id: [0x22; 16],
        requested_mutations: vec![],
        max_batch_items: 5,
    };
    let batches = AntiEntropyEngine::generate_batches(&node_src, &req, &[]);
    assert_eq!(batches.len(), 2);

    // Ingest Batch 1 FIRST (out of order), then Batch 0
    AntiEntropyEngine::ingest_batch(&mut node_dst, batches[1].clone()).unwrap();
    AntiEntropyEngine::ingest_batch(&mut node_dst, batches[0].clone()).unwrap();

    assert_eq!(node_dst.state.state_node.dag.len(), 10, "All 10 items must be reassembled despite out-of-order ingress");

    node_src.stop().unwrap();
    node_dst.stop().unwrap();
}

#[test]
fn test_r50_4_e_malicious_forged_mutation_rejection() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    node.create_object([0x66; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"AUTHENTIC".to_vec()).unwrap();
    let m = node.state.state_node.dag.values().next().unwrap().clone();

    // Forge mutation ID
    let mut forged_mutation = m.clone();
    forged_mutation.id = [0x99; 32]; // ID does not match hash(body)

    let bad_batch = SyncStreamBatch {
        session_id: [0x33; 16],
        batch_index: 0,
        total_batches: 1,
        mutations: vec![forged_mutation],
    };

    let res = AntiEntropyEngine::ingest_batch(&mut node, bad_batch);
    assert!(res.is_err(), "Forged mutation in batch must be rejected with preflight verification error");

    node.stop().unwrap();
}

#[test]
fn test_r50_4_f_resume_interrupted_synchronization() {
    let tmp_src = tempdir().unwrap();
    let tmp_dst = tempdir().unwrap();
    let mut csprng = OsRng;
    let key_src = SigningKey::generate(&mut csprng);
    let key_dst = SigningKey::generate(&mut csprng);

    let mut node_src = NexNode::new(tmp_src.path(), key_src);
    let mut node_dst = NexNode::new(tmp_dst.path(), key_dst);
    node_src.start().unwrap();
    node_dst.start().unwrap();

    for i in 0..20 {
        node_src.create_object([0x77; 32], ObjectType::Synthetic(1), BTreeMap::new(), format!("RESUME_{}", i).into_bytes()).unwrap();
    }

    let session1 = [0x44; 16];
    let req1 = nex_core::sync::anti_entropy::SyncDeltaRequest {
        session_id: session1,
        requested_mutations: vec![],
        max_batch_items: 10,
    };
    let batches = AntiEntropyEngine::generate_batches(&node_src, &req1, &[]);

    // 1. Deliver Batch 0 only
    AntiEntropyEngine::ingest_batch(&mut node_dst, batches[0].clone()).unwrap();
    assert_eq!(node_dst.state.state_node.dag.len(), 10);

    // 2. Disconnect & Reconnect (Session 2)
    let session2 = [0x45; 16];
    let adv_dst2 = AntiEntropyEngine::generate_advertise(&mut node_dst, session2);
    assert!(AntiEntropyEngine::has_deltas_for_peer(&node_src, &adv_dst2));

    let resume_batches = AntiEntropyEngine::generate_batches_for_peer(&node_src, session2, &adv_dst2.frontier_mutation_ids, 10);
    assert_eq!(resume_batches.len(), 1, "Resume must only generate 1 remaining batch for the 10 unsynced mutations");

    AntiEntropyEngine::ingest_batch(&mut node_dst, resume_batches[0].clone()).unwrap();
    assert_eq!(node_dst.state.state_node.dag.len(), 20, "Total 20 mutations after resume");

    node_src.stop().unwrap();
    node_dst.stop().unwrap();
}

#[test]
fn test_r50_4_g_out_of_order_mutation_crdt_lww_object_store_consistency() {
    use nex_core::model::{Mutation, MutationBody, CrdtPayload};
    use nex_core::hash::hash_mutation_body;

    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let target_obj_id = [0xAA; 32];
    let other_obj_id = [0xBB; 32];

    // Genesis mutation for common causal ancestor (Lamport 0, Epoch 0)
    let body_genesis = MutationBody {
        author: [0x01; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target_obj_id,
            value: b"GENESIS_VALUE".to_vec(),
        },
    };
    let m_genesis = Mutation::new(hash_mutation_body(&body_genesis), body_genesis);

    // Branch A (Causally Earlier branch): Lamport 1, Epoch 0
    let body_earlier = MutationBody {
        author: [0x01; 32],
        parents: vec![m_genesis.id],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target_obj_id,
            value: b"OBSOLETE_EARLIER_VALUE".to_vec(),
        },
    };
    let m_earlier = Mutation::new(hash_mutation_body(&body_earlier), body_earlier);

    // Branch B (Causally Later branch):
    // 1. Intermediate mutation (Lamport 1)
    let body_inter = MutationBody {
        author: [0x02; 32],
        parents: vec![m_genesis.id],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: other_obj_id,
            value: b"INTERMEDIATE".to_vec(),
        },
    };
    let m_inter = Mutation::new(hash_mutation_body(&body_inter), body_inter);

    // 2. Later winning mutation (Lamport 2) on target_obj_id
    let body_later = MutationBody {
        author: [0x02; 32],
        parents: vec![m_inter.id],
        lamport: 2,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target_obj_id,
            value: b"WINNING_LATER_VALUE".to_vec(),
        },
    };
    let m_later = Mutation::new(hash_mutation_body(&body_later), body_later);

    // Step 1: Ingest Genesis
    let batch_genesis = SyncStreamBatch {
        session_id: [0x99; 16],
        batch_index: 0,
        total_batches: 3,
        mutations: vec![m_genesis],
    };
    AntiEntropyEngine::ingest_batch(&mut node, batch_genesis).unwrap();

    // Step 2: Ingest Causally Later branch (m_inter, m_later) FIRST
    let batch_later = SyncStreamBatch {
        session_id: [0x99; 16],
        batch_index: 1,
        total_batches: 3,
        mutations: vec![m_inter, m_later.clone()],
    };
    AntiEntropyEngine::ingest_batch(&mut node, batch_later).unwrap();

    // Verify crdt_state and object_store currently hold WINNING_LATER_VALUE
    let crdt_entry_init = node.state.state_node.crdt_state.get(&target_obj_id).unwrap();
    assert_eq!(crdt_entry_init.3, m_later.id);
    let obj_init = node.state.object_store.get(&target_obj_id).expect("Object must exist in object_store");
    assert_eq!(obj_init.payload_bytes, b"WINNING_LATER_VALUE".to_vec());

    // Step 3: Ingest Causally Earlier mutation (m_earlier) SECOND (out of causal order)
    let batch_earlier = SyncStreamBatch {
        session_id: [0x99; 16],
        batch_index: 2,
        total_batches: 3,
        mutations: vec![m_earlier],
    };
    AntiEntropyEngine::ingest_batch(&mut node, batch_earlier).unwrap();

    // In crdt_state, m_later MUST WIN because Lamport 2 > Lamport 1
    let crdt_entry = node.state.state_node.crdt_state.get(&target_obj_id).unwrap();
    assert_eq!(crdt_entry.3, m_later.id, "crdt_state must retain m_later as winner");

    // REGRESSION ASSERTION:
    // object_store MUST ALSO retain WINNING_LATER_VALUE, NOT get overwritten by obsolete m_earlier!
    let obj_final = node.state.object_store.get(&target_obj_id).expect("Object must exist in object_store");
    assert_eq!(
        obj_final.payload_bytes,
        b"WINNING_LATER_VALUE".to_vec(),
        "object_store must reflect the causal LWW winner regardless of batch arrival order"
    );

    node.stop().unwrap();
}

#[test]
fn test_r50_4_h_targeted_batch_ingest_performance_and_isolation() {
    use std::time::Instant;
    use nex_core::model::{Mutation, MutationBody, CrdtPayload};
    use nex_core::hash::hash_mutation_body;
    use nex_core::object::types::NexObject;

    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let mut object_ids = Vec::with_capacity(5000);

    // 1. Populate object_store and state_node with 5,000 baseline objects in memory
    for i in 0..5000 {
        let mut obj_id = [0u8; 32];
        obj_id[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        obj_id[30] = 0xAA;
        object_ids.push(obj_id);

        let body = MutationBody {
            author: [0x01; 32],
            parents: vec![],
            lamport: 0,
            epoch: 0,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW {
                id: obj_id,
                value: format!("PAYLOAD_{}", i).into_bytes(),
            },
        };
        let m = Mutation::new(hash_mutation_body(&body), body);
        let m_id = m.id;
        node.state.state_node.crdt_state.insert(obj_id, (Some(format!("PAYLOAD_{}", i).into_bytes()), 0, 0, m_id));
        node.state.state_node.dag.insert(m_id, m);

        let obj = NexObject {
            object_id: obj_id,
            object_type: ObjectType::Synthetic(1),
            namespace: [0x10; 32],
            owner_actor_id: [0x01; 32],
            schema_version: 1,
            created_epoch: 0,
            created_lamport: 0,
            winning_mutation_id: m_id,
            metadata: BTreeMap::new(),
            payload_bytes: format!("PAYLOAD_{}", i).into_bytes(),
            tombstoned: false,
        };
        node.state.object_store.insert(obj_id, obj);
    }
    assert_eq!(node.state.object_store.len(), 5000);

    // Pick 2 specific targets out of the 5,000
    let target1_id = object_ids[42];
    let target1_obj = node.state.object_store.get(&target1_id).unwrap().clone();

    let target2_id = object_ids[4200];
    let target2_obj = node.state.object_store.get(&target2_id).unwrap().clone();

    // Pick an untouched reference object
    let ref_id = object_ids[999];
    let ref_payload_before = node.state.object_store.get(&ref_id).unwrap().payload_bytes.clone();

    // 2. Prepare a targeted 3-mutation batch modifying only target1 and target2, plus 1 new object
    let body1 = MutationBody {
        author: [0x01; 32],
        parents: vec![target1_obj.winning_mutation_id],
        lamport: target1_obj.created_lamport + 1,
        epoch: target1_obj.created_epoch,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target1_id,
            value: b"UPDATED_TARGET_1".to_vec(),
        },
    };
    let m1 = Mutation::new(hash_mutation_body(&body1), body1);

    let body2 = MutationBody {
        author: [0x01; 32],
        parents: vec![target2_obj.winning_mutation_id],
        lamport: target2_obj.created_lamport + 1,
        epoch: target2_obj.created_epoch,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target2_id,
            value: b"UPDATED_TARGET_2".to_vec(),
        },
    };
    let m2 = Mutation::new(hash_mutation_body(&body2), body2);

    let new_obj_id = [0xEE; 32];
    let body3 = MutationBody {
        author: [0x01; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: new_obj_id,
            value: b"BRAND_NEW_OBJECT".to_vec(),
        },
    };
    let m3 = Mutation::new(hash_mutation_body(&body3), body3);

    let batch = SyncStreamBatch {
        session_id: [0x88; 16],
        batch_index: 0,
        total_batches: 1,
        mutations: vec![m1, m2, m3],
    };

    // 3. Measure execution time of targeted ingest_batch on 5,000-object store
    let start_time = Instant::now();
    let ack = AntiEntropyEngine::ingest_batch(&mut node, batch).expect("Targeted batch ingest must succeed");
    let elapsed = start_time.elapsed();

    assert_eq!(ack.ingested_count, 3);
    // Assert sub-250ms execution in unoptimized debug mode demonstrating targeted O(1) update rather than O(objects * mutations)
    assert!(elapsed.as_millis() < 250, "Targeted batch ingest took {:?} which exceeds 250ms SLA", elapsed);

    // 4. Assert isolation and correctness
    assert_eq!(node.state.object_store.get(&target1_id).unwrap().payload_bytes, b"UPDATED_TARGET_1".to_vec());
    assert_eq!(node.state.object_store.get(&target2_id).unwrap().payload_bytes, b"UPDATED_TARGET_2".to_vec());
    assert_eq!(node.state.object_store.get(&new_obj_id).unwrap().payload_bytes, b"BRAND_NEW_OBJECT".to_vec());
    assert_eq!(node.state.object_store.get(&ref_id).unwrap().payload_bytes, ref_payload_before, "Untouched object must remain unmodified");
    assert_eq!(node.state.object_store.len(), 5001);

    node.stop().unwrap();
}

#[test]
fn test_r50_4_i_full_object_deterministic_tiebreak() {
    use nex_core::object::store::NexObjectStore;
    use nex_core::object::types::{NexObject, ObjectType};

    let obj_id = [0x55; 32];
    let winning_mut_low = [0x10; 32];
    let winning_mut_high = [0x20; 32];

    let obj_low = NexObject {
        object_id: obj_id,
        object_type: ObjectType::Synthetic(1),
        namespace: [0x01; 32],
        owner_actor_id: [0x0A; 32],
        schema_version: 1,
        created_epoch: 2,
        created_lamport: 10,
        winning_mutation_id: winning_mut_low,
        metadata: BTreeMap::new(),
        payload_bytes: b"LOW_MUTATION_VALUE".to_vec(),
        tombstoned: false,
    };

    let obj_high = NexObject {
        object_id: obj_id,
        object_type: ObjectType::Synthetic(1),
        namespace: [0x01; 32],
        owner_actor_id: [0x0B; 32],
        schema_version: 1,
        created_epoch: 2,
        created_lamport: 10,
        winning_mutation_id: winning_mut_high,
        metadata: BTreeMap::new(),
        payload_bytes: b"HIGH_MUTATION_VALUE".to_vec(),
        tombstoned: false,
    };

    // Scenario 1: Low inserted first, High inserted second -> High MUST WIN
    let mut store1 = NexObjectStore::new();
    store1.insert(obj_low.clone());
    store1.insert(obj_high.clone());
    assert_eq!(store1.get(&obj_id).unwrap().payload_bytes, b"HIGH_MUTATION_VALUE".to_vec());
    assert_eq!(store1.get(&obj_id).unwrap().winning_mutation_id, winning_mut_high);

    // Scenario 2: High inserted first, Low inserted second -> High MUST STILL WIN (Deterministic tiebreak)
    let mut store2 = NexObjectStore::new();
    store2.insert(obj_high);
    store2.insert(obj_low);
    assert_eq!(
        store2.get(&obj_id).unwrap().payload_bytes,
        b"HIGH_MUTATION_VALUE".to_vec(),
        "Higher lexicographic winning_mutation_id must win regardless of arrival order"
    );
    assert_eq!(store2.get(&obj_id).unwrap().winning_mutation_id, winning_mut_high);
}

#[test]
fn test_r50_4_j_wal_replay_out_of_order_crdt_lww_consistency() {
    use nex_core::model::{Mutation, MutationBody, CrdtPayload};
    use nex_core::hash::hash_mutation_body;
    use nex_core::storage::wal::WriteAheadLog;

    let tmp = tempdir().unwrap();
    let wal_path = tmp.path().join("wal.log");

    let target_obj_id = [0x77; 32];
    let inter_obj_id = [0x88; 32];

    // Genesis (Lamport 0)
    let body_genesis = MutationBody {
        author: [0x01; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target_obj_id,
            value: b"GENESIS_WAL".to_vec(),
        },
    };
    let m_genesis = Mutation::new(hash_mutation_body(&body_genesis), body_genesis);

    // Intermediate mutation (Lamport 1) on inter_obj_id
    let body_inter = MutationBody {
        author: [0x02; 32],
        parents: vec![m_genesis.id],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: inter_obj_id,
            value: b"INTERMEDIATE".to_vec(),
        },
    };
    let m_inter = Mutation::new(hash_mutation_body(&body_inter), body_inter);

    // Causally Later winning mutation (Lamport 2) on target_obj_id
    let body_later = MutationBody {
        author: [0x02; 32],
        parents: vec![m_inter.id],
        lamport: 2,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target_obj_id,
            value: b"WINNING_WAL_VALUE".to_vec(),
        },
    };
    let m_later = Mutation::new(hash_mutation_body(&body_later), body_later);

    // Obsolete earlier mutation (Lamport 1) on target_obj_id
    let body_earlier = MutationBody {
        author: [0x01; 32],
        parents: vec![m_genesis.id],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: target_obj_id,
            value: b"OBSOLETE_EARLIER_WAL".to_vec(),
        },
    };
    let m_earlier = Mutation::new(hash_mutation_body(&body_earlier), body_earlier);

    // 1. Write mutations to wal.log in OUT-OF-ORDER sequence:
    // Genesis -> Intermediate -> Causally Later -> Obsolete Earlier (written last to disk)
    {
        let mut wal = WriteAheadLog::open(&wal_path).unwrap();
        wal.append_mutation(&m_genesis).unwrap();
        wal.append_mutation(&m_inter).unwrap();
        wal.append_mutation(&m_later).unwrap();
        wal.append_mutation(&m_earlier).unwrap(); // Obsolete written after later!
    }

    // 2. Start fresh NexNode on this directory (triggers WAL replay recovery)
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().expect("Node crash recovery via WAL replay must succeed");

    // 3. Assert CRDT state and ObjectStore convergence after WAL recovery
    let crdt_entry = node.state.state_node.crdt_state.get(&target_obj_id).unwrap();
    assert_eq!(crdt_entry.3, m_later.id, "crdt_state must resolve to m_later after WAL replay");

    let obj = node.state.object_store.get(&target_obj_id).expect("target object must exist in object_store");
    assert_eq!(
        obj.payload_bytes,
        b"WINNING_WAL_VALUE".to_vec(),
        "object_store must reflect the winning LWW mutation after WAL recovery replay"
    );
    assert_eq!(obj.winning_mutation_id, m_later.id);

    node.stop().unwrap();
}
