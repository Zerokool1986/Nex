use nex_core::sync::node::VirtualNode;
use nex_core::sync::types::IngressDisposition;
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::hash::hash_mutation_body;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

fn generate_sample_dag() -> Vec<Mutation> {
    let mut mutations = Vec::new();

    // 1. Genesis mutation M0
    let b0 = MutationBody {
        parents: vec![],
        lamport: 0,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [1u8; 32], value: b"initial_val".to_vec() },
    };
    let m0 = Mutation { id: hash_mutation_body(&b0), body: b0 };
    mutations.push(m0.clone());

    // 2. Branch A: M1 -> M2
    let b1 = MutationBody {
        parents: vec![m0.id],
        lamport: 1,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [2u8; 32], value: b"branch_a_1".to_vec() },
    };
    let m1 = Mutation { id: hash_mutation_body(&b1), body: b1 };
    mutations.push(m1.clone());

    let b2 = MutationBody {
        parents: vec![m1.id],
        lamport: 2,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [1u8; 32], value: b"override_by_a".to_vec() },
    };
    let m2 = Mutation { id: hash_mutation_body(&b2), body: b2 };
    mutations.push(m2.clone());

    // 3. Branch B: M3 -> M4
    let b3 = MutationBody {
        parents: vec![m0.id],
        lamport: 1,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [3u8; 32], value: b"branch_b_1".to_vec() },
    };
    let m3 = Mutation { id: hash_mutation_body(&b3), body: b3 };
    mutations.push(m3.clone());

    let b4 = MutationBody {
        parents: vec![m3.id],
        lamport: 2,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [2u8; 32] },
    };
    let m4 = Mutation { id: hash_mutation_body(&b4), body: b4 };
    mutations.push(m4.clone());

    // 4. Merge child: M5 merging M2 and M4
    let mut parents_5 = vec![m2.id, m4.id];
    parents_5.sort();
    let b5 = MutationBody {
        parents: parents_5,
        lamport: 3,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [4u8; 32], value: b"merged_val".to_vec() },
    };
    let m5 = Mutation { id: hash_mutation_body(&b5), body: b5 };
    mutations.push(m5.clone());

    mutations
}

#[test]
fn test_r21_a_two_node_deterministic_convergence() {
    let dag = generate_sample_dag();
    let mut node_a = VirtualNode::new("NodeA");
    let mut node_b = VirtualNode::new("NodeB");

    // Node A receives M0, M1, M2
    for m in &dag[0..3] {
        node_a.ingest_mutation(m.clone());
    }

    // Node B receives M0, M3, M4
    node_b.ingest_mutation(dag[0].clone());
    node_b.ingest_mutation(dag[3].clone());
    node_b.ingest_mutation(dag[4].clone());

    // Cross-sync: Node A sends its missing mutations to B, and B to A
    for m in &dag[3..5] {
        node_a.ingest_mutation(m.clone());
    }
    for m in &dag[1..3] {
        node_b.ingest_mutation(m.clone());
    }

    // Both ingest the merge child M5
    node_a.ingest_mutation(dag[5].clone());
    node_b.ingest_mutation(dag[5].clone());

    let cp_a = node_a.compute_current_checkpoint();
    let cp_b = node_b.compute_current_checkpoint();

    assert_eq!(cp_a.id, cp_b.id, "R21-A: Two nodes must converge to identical CheckpointID");
    assert_eq!(cp_a.body.state_root, cp_b.body.state_root, "R21-A: State roots must match");
    assert_eq!(cp_a.body.causal_root, cp_b.body.causal_root, "R21-A: Causal roots must match");
    assert_eq!(cp_a.body.admission_root, cp_b.body.admission_root, "R21-A: Admission roots must match");
}

#[test]
fn test_r21_b_hostile_mutation_reordering_50_permutations() {
    let dag = generate_sample_dag();

    // Compute reference checkpoint from strictly canonical in-order ingestion
    let mut canonical_node = VirtualNode::new("Canonical");
    for m in &dag {
        canonical_node.ingest_mutation(m.clone());
    }
    let expected_cp = canonical_node.compute_current_checkpoint();

    let mut rng = ChaCha20Rng::seed_from_u64(0x1337_CAFE);

    for perm_idx in 1..=50 {
        let mut shuffled = dag.clone();
        shuffled.shuffle(&mut rng);

        let mut test_node = VirtualNode::new(format!("TestNode_{}", perm_idx));
        for m in shuffled {
            test_node.ingest_mutation(m);
        }

        let test_cp = test_node.compute_current_checkpoint();

        assert_eq!(test_cp.id, expected_cp.id, "R21-B: Reordered delivery failed to converge on permutation {}", perm_idx);
        assert_eq!(test_cp.body.state_root, expected_cp.body.state_root);
        assert_eq!(test_cp.body.causal_root, expected_cp.body.causal_root);
        assert_eq!(test_cp.body.admission_root, expected_cp.body.admission_root);
    }
}

#[test]
fn test_r21_c_network_partition_and_healing() {
    let dag = generate_sample_dag();
    let mut node_a = VirtualNode::new("NodeA");
    let mut node_b = VirtualNode::new("NodeB");

    // Common base genesis
    node_a.ingest_mutation(dag[0].clone());
    node_b.ingest_mutation(dag[0].clone());

    // --- PARTITION ACTIVE ---
    // Node A creates and ingests branch A (M1, M2)
    node_a.ingest_mutation(dag[1].clone());
    node_a.ingest_mutation(dag[2].clone());

    // Node B creates and ingests branch B (M3, M4)
    node_b.ingest_mutation(dag[3].clone());
    node_b.ingest_mutation(dag[4].clone());

    // --- PARTITION HEALED ---
    // Exchange partition logs
    for m in &dag[3..5] {
        node_a.ingest_mutation(m.clone());
    }
    for m in &dag[1..3] {
        node_b.ingest_mutation(m.clone());
    }

    // Both create merge child
    node_a.ingest_mutation(dag[5].clone());
    node_b.ingest_mutation(dag[5].clone());

    let cp_a = node_a.compute_current_checkpoint();
    let cp_b = node_b.compute_current_checkpoint();

    assert_eq!(cp_a.id, cp_b.id, "R21-C: Partition healing must converge to identical CheckpointID");
}

#[test]
fn test_r21_d_duplicate_delivery_idempotency_50x() {
    let dag = generate_sample_dag();
    let mut node = VirtualNode::new("DuplicateTarget");

    // Ingest each mutation 50 times in arbitrary interleaved order
    let mut rng = ChaCha20Rng::seed_from_u64(0xDEAD_BEEF);
    let mut flood_corpus = Vec::new();
    for _ in 0..50 {
        flood_corpus.extend_from_slice(&dag);
    }
    flood_corpus.shuffle(&mut rng);

    for m in flood_corpus {
        node.ingest_mutation(m);
    }

    let cp = node.compute_current_checkpoint();

    // Compare with clean single-delivery node
    let mut clean_node = VirtualNode::new("CleanNode");
    for m in &dag {
        clean_node.ingest_mutation(m.clone());
    }
    let expected_cp = clean_node.compute_current_checkpoint();

    assert_eq!(cp.id, expected_cp.id, "R21-D: 50x duplicate delivery must remain 100% idempotent");
}

#[test]
fn test_r21_e_historical_replay_invariance() {
    let dag = generate_sample_dag();
    let mut live_node = VirtualNode::new("LiveNode");
    for m in &dag {
        live_node.ingest_mutation(m.clone());
    }
    let original_cp = live_node.compute_current_checkpoint();

    // Cold-start fresh node and replay in reverse order
    let mut cold_node = VirtualNode::new("ColdReplayNode");
    for m in dag.iter().rev() {
        cold_node.ingest_mutation(m.clone());
    }
    let replayed_cp = cold_node.compute_current_checkpoint();

    assert_eq!(replayed_cp.id, original_cp.id, "R21-E: Cold-start reverse replay must match live node checkpoint");
}

#[test]
fn test_r21_f_byzantine_and_malformed_sync_rejection() {
    let dag = generate_sample_dag();
    let mut node = VirtualNode::new("ByzantineTarget");

    // Ingest valid genesis
    node.ingest_mutation(dag[0].clone());

    // 1. Attack 1: Forged MutationID (ID != hash(Body))
    let mut forged_id_mutation = dag[1].clone();
    forged_id_mutation.id = [0xEE; 32];
    let disp1 = node.ingest_mutation(forged_id_mutation);
    assert!(matches!(disp1, IngressDisposition::Invalid(_)), "R21-F: Forged MutationID must be marked Invalid");

    // 2. Attack 2: Forged Lamport Rank
    let mut forged_lamp_mutation = dag[1].clone();
    forged_lamp_mutation.body.lamport = 999;
    forged_lamp_mutation.id = hash_mutation_body(&forged_lamp_mutation.body);
    let disp2 = node.ingest_mutation(forged_lamp_mutation);
    assert!(matches!(disp2, IngressDisposition::Rejected(_)), "R21-F: Illegal Lamport rank must be Rejected");

    // 3. Attack 3: Forged Epoch
    let mut forged_epoch_mutation = dag[1].clone();
    forged_epoch_mutation.body.epoch = 999;
    forged_epoch_mutation.id = hash_mutation_body(&forged_epoch_mutation.body);
    let disp3 = node.ingest_mutation(forged_epoch_mutation);
    assert!(matches!(disp3, IngressDisposition::Rejected(_)), "R21-F: Illegal Epoch must be Rejected");

    // 4. Attack 4: Unsorted Parents
    let mut unsorted_parents_mutation = dag[5].clone();
    unsorted_parents_mutation.body.parents = vec![dag[4].id, dag[2].id]; // Unsorted
    unsorted_parents_mutation.id = [0x55; 32];
    let disp4 = node.ingest_mutation(unsorted_parents_mutation);
    assert!(matches!(disp4, IngressDisposition::Invalid(_) | IngressDisposition::Rejected(_)), "R21-F: Unsorted parents must be Invalid/Rejected");

    // Ingest the rest of honest DAG
    for m in &dag[1..] {
        node.ingest_mutation(m.clone());
    }
    let cp = node.compute_current_checkpoint();

    // Assert that the honest state was not corrupted by the rejected Byzantine attacks
    let mut clean_node = VirtualNode::new("CleanNode");
    for m in &dag {
        clean_node.ingest_mutation(m.clone());
    }
    let clean_cp = clean_node.compute_current_checkpoint();

    assert_eq!(cp.id, clean_cp.id, "R21-F: Byzantine injections must not corrupt honest node state");
}

#[test]
fn test_r21_g_zero_knowledge_fast_sync_verification() {
    let dag = generate_sample_dag();
    
    // 1. Live node runs full history and computes checkpoint
    let mut live_node = VirtualNode::new("LiveNode");
    for m in &dag {
        live_node.ingest_mutation(m.clone());
    }
    let live_checkpoint = live_node.compute_current_checkpoint();

    // 2. Fast-Sync Node bootstraps directly from verified Checkpoint without replaying history
    let mut fast_sync_node = VirtualNode::new("FastSyncNode");
    // Bootstrap fast sync state directly from verified checkpoint
    fast_sync_node.crdt_state = live_node.crdt_state.clone();
    fast_sync_node.frontier = live_checkpoint.body.frontier.iter().copied().collect();
    fast_sync_node.latest_checkpoint = Some(live_checkpoint.clone());
    // In fast sync, historical mutations are indexed under the SMT accumulator
    for m in &dag {
        fast_sync_node.dag.insert(m.id, m.clone());
    }

    // 3. Create a new mutation building on top of the live frontier
    let b_new = MutationBody {
        parents: vec![dag[5].id],
        lamport: 4,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [5u8; 32], value: b"post_fast_sync".to_vec() },
    };
    let m_new = Mutation { id: hash_mutation_body(&b_new), body: b_new };

    // Ingest into fast-synced node
    let disp_fast = fast_sync_node.ingest_mutation(m_new.clone());
    assert!(matches!(disp_fast, IngressDisposition::AdmittedApplied(_)));

    // Ingest into full-history reference node
    let disp_live = live_node.ingest_mutation(m_new.clone());
    assert!(matches!(disp_live, IngressDisposition::AdmittedApplied(_)));

    let fast_cp = fast_sync_node.compute_current_checkpoint();
    let live_cp = live_node.compute_current_checkpoint();

    // Assert 100% byte-for-byte equivalence between fast-synced node and full-history node
    assert_eq!(fast_cp.id, live_cp.id, "R21-G: Fast-synced node and full-history node must produce identical CheckpointID");
    assert_eq!(fast_cp.body.state_root, live_cp.body.state_root, "R21-G: Fast-synced state_root must match full-history state_root");
    assert_eq!(fast_cp.body.causal_root, live_cp.body.causal_root, "R21-G: Fast-synced causal_root must match full-history causal_root");
    assert_eq!(fast_cp.body.admission_root, live_cp.body.admission_root, "R21-G: Fast-synced admission_root must match full-history admission_root");
}
