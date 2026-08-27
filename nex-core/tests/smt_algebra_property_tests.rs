use nex_core::accumulator::{SparseMerkleTree, SmtUpdateResult};
use nex_core::model::StateEncoding;
use nex_core::hash::hash_state_encoding;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

fn generate_random_mutations(rng: &mut ChaCha20Rng, count: usize) -> Vec<([u8; 32], [u8; 32])> {
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let mut m_id = [0u8; 32];
        rng.fill_bytes(&mut m_id);
        
        let state = StateEncoding {
            mutation_id: m_id,
            lamport: (i + 1) as u64,
            epoch: 0,
            is_resurrect: false,
            payload: nex_core::model::CrdtPayload::AddLWW {
                id: [0x42; 32],
                value: vec![i as u8],
            },
        };
        let commitment = hash_state_encoding(&state);
        entries.push((m_id, commitment));
    }
    entries
}

#[test]
fn test_r15_a_smt_commutativity_property() {
    let mut rng = ChaCha20Rng::seed_from_u64(0xDEADBEEF);
    
    // Repeat for 20 independent randomized runs
    for _ in 0..20 {
        let set_a = generate_random_mutations(&mut rng, 15);
        let set_b = generate_random_mutations(&mut rng, 15);

        // Path 1: Insert A then B
        let mut smt_1 = SparseMerkleTree::new();
        for (m_id, comm) in &set_a {
            let res = smt_1.insert_or_verify(m_id, comm).unwrap();
            assert!(matches!(res, SmtUpdateResult::Inserted(_)));
        }
        for (m_id, comm) in &set_b {
            let res = smt_1.insert_or_verify(m_id, comm).unwrap();
            assert!(matches!(res, SmtUpdateResult::Inserted(_)));
        }
        let root_ab = smt_1.root();

        // Path 2: Insert B then A
        let mut smt_2 = SparseMerkleTree::new();
        for (m_id, comm) in &set_b {
            let res = smt_2.insert_or_verify(m_id, comm).unwrap();
            assert!(matches!(res, SmtUpdateResult::Inserted(_)));
        }
        for (m_id, comm) in &set_a {
            let res = smt_2.insert_or_verify(m_id, comm).unwrap();
            assert!(matches!(res, SmtUpdateResult::Inserted(_)));
        }
        let root_ba = smt_2.root();

        assert_eq!(root_ab, root_ba, "SMT accumulation must be strictly commutative!");
    }
}

#[test]
fn test_r15_a_smt_idempotence_and_noop_property() {
    let mut rng = ChaCha20Rng::seed_from_u64(0xCAFEFEED);
    let mut smt = SparseMerkleTree::new();
    let entries = generate_random_mutations(&mut rng, 30);

    for (m_id, comm) in &entries {
        let res = smt.insert_or_verify(m_id, comm).unwrap();
        assert!(matches!(res, SmtUpdateResult::Inserted(_)));
    }
    let root_before_replays = smt.root();

    // Replay all entries again
    for (m_id, comm) in &entries {
        let res = smt.insert_or_verify(m_id, comm).unwrap();
        assert!(matches!(res, SmtUpdateResult::NoOp(_)));
    }
    let root_after_replays = smt.root();

    assert_eq!(root_before_replays, root_after_replays, "SMT accumulation must be strictly idempotent!");
}

#[test]
fn test_r15_a_smt_fork_merge_convergence_property() {
    let mut rng = ChaCha20Rng::seed_from_u64(0x12345678);

    // Shared common history (LCA)
    let common_history = generate_random_mutations(&mut rng, 10);
    let branch_a = generate_random_mutations(&mut rng, 12);
    let branch_b = generate_random_mutations(&mut rng, 12);

    // Peer 1 accumulates Common + Branch A
    let mut peer_1_smt = SparseMerkleTree::new();
    for (m, c) in &common_history { peer_1_smt.insert_or_verify(m, c).unwrap(); }
    for (m, c) in &branch_a { peer_1_smt.insert_or_verify(m, c).unwrap(); }

    // Peer 2 accumulates Common + Branch B
    let mut peer_2_smt = SparseMerkleTree::new();
    for (m, c) in &common_history { peer_2_smt.insert_or_verify(m, c).unwrap(); }
    for (m, c) in &branch_b { peer_2_smt.insert_or_verify(m, c).unwrap(); }

    // Fork Reconciliation: Peer 1 receives Branch B; Peer 2 receives Branch A
    for (m, c) in &branch_b { peer_1_smt.insert_or_verify(m, c).unwrap(); }
    for (m, c) in &branch_a { peer_2_smt.insert_or_verify(m, c).unwrap(); }

    assert_eq!(
        peer_1_smt.root(),
        peer_2_smt.root(),
        "Both peers must deterministically converge to the exact same SMT root!"
    );
}

#[test]
fn test_r15_a_smt_conflict_rejection_property() {
    let mut smt = SparseMerkleTree::new();
    let m_id = [0x55; 32];
    let commitment_1 = [0x11; 32];
    let commitment_2 = [0x22; 32];

    let res1 = smt.insert_or_verify(&m_id, &commitment_1).unwrap();
    assert!(matches!(res1, SmtUpdateResult::Inserted(_)));

    // Attempting to insert a conflicting StateCommitment for the same MutationID must return Conflict
    let res2 = smt.insert_or_verify(&m_id, &commitment_2).unwrap();
    assert_eq!(res2, SmtUpdateResult::Conflict);
}
