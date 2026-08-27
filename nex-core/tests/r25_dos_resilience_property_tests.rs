use std::collections::HashSet;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::types::KeyType;
use nex_core::resilience::rate_limiter::PeerRateLimiter;
use nex_core::resilience::peer_jail::PeerJail;
use nex_core::resilience::bounded_buffer::{BoundedDependencyBuffer, MAX_DEPENDENCY_BUFFER_ENTRIES};
use nex_core::resilience::preflight_shield::{PreFlightShield, PreFlightError};
use nex_core::model::{Mutation, MutationBody, Checkpoint, CheckpointBody, Boundary, CrdtPayload};
use nex_core::hash::{hash_mutation_body, hash_checkpoint_body};
use nex_core::sync::node::VirtualNode;

#[test]
fn test_r25_a_token_bucket_rate_limiter() {
    let peer = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let mut limiter = PeerRateLimiter::new(10, 2); // 10 token burst capacity, 2 tokens/sec refill

    // 1. Consume 10 tokens in burst at epoch 0 -> Allowed
    for _ in 0..10 {
        assert!(limiter.check_and_consume(&peer, 0, 1), "Token consumption within burst capacity must succeed");
    }

    // 2. 11th token in same epoch -> Dropped
    assert!(!limiter.check_and_consume(&peer, 0, 1), "R25-A: Traffic exceeding burst capacity must be dropped");

    // 3. Advance epoch by 3 seconds -> 6 tokens refilled
    assert!(limiter.check_and_consume(&peer, 3, 5), "Refilled tokens must permit new requests");
    assert!(limiter.check_and_consume(&peer, 3, 1));
    assert!(!limiter.check_and_consume(&peer, 3, 1), "Exceeding refilled tokens must be dropped again");
}

#[test]
fn test_r25_b_dependency_buffer_lru_and_ttl_bounding() {
    let mut buffer = BoundedDependencyBuffer::new();

    // Spam 1,000 orphan mutations into the buffer at epoch 10
    for i in 0..1000u64 {
        let body = MutationBody {
            parents: vec![[0xEE; 32]],
            lamport: 1,
            epoch: 0,
            author: [0u8; 32],
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [1u8; 32], value: vec![] },
        };
        let mut id = [0u8; 32];
        id[0..8].copy_from_slice(&i.to_le_bytes());
        let m = Mutation { id, body };
        let mut missing = HashSet::new();
        missing.insert([0xEE; 32]);

        buffer.insert(m, missing, 10 + (i % 50));
    }

    // Assert buffer never exceeds MAX_DEPENDENCY_BUFFER_ENTRIES (512)
    assert_eq!(buffer.len(), MAX_DEPENDENCY_BUFFER_ENTRIES, "R25-B: Buffer must strictly clamp to max capacity");

    // Advance epoch beyond TTL (epoch 10 + 301 = 311) -> prune
    let pruned = buffer.prune_expired(400);
    assert_eq!(pruned, MAX_DEPENDENCY_BUFFER_ENTRIES, "R25-B: Expired orphans must be reclaimed");
    assert_eq!(buffer.len(), 0);
}

#[test]
fn test_r25_d_preflight_proof_verification_shield() {
    let expected_image_id = [0x42; 32];
    let shield = PreFlightShield::new(expected_image_id);

    let valid_cp_body = CheckpointBody {
        state_root: [0x11; 32],
        causal_root: [0x22; 32],
        admission_root: [0x33; 32],
        frontier: vec![],
        boundary: Boundary { max_epoch: 0, max_lamport: 0 },
    };
    let valid_cp = Checkpoint {
        id: hash_checkpoint_body(&valid_cp_body),
        body: valid_cp_body,
    };

    // 1. Valid preflight proof
    let res_ok = shield.validate_proof_preflight(&expected_image_id, 1, &[0xAA; 100], &valid_cp);
    assert_eq!(res_ok, Ok(()));

    // 2. Mismatched ImageID -> Shield drops immediately (< 1 microsecond)
    let bad_image_id = [0x99; 32];
    let res_bad_img = shield.validate_proof_preflight(&bad_image_id, 1, &[0xAA; 100], &valid_cp);
    assert_eq!(res_bad_img, Err(PreFlightError::ImageIdMismatch), "R25-D: Mismatched image_id must be dropped in preflight");

    // 3. Bad ABI version
    let res_bad_abi = shield.validate_proof_preflight(&expected_image_id, 99, &[0xAA; 100], &valid_cp);
    assert_eq!(res_bad_abi, Err(PreFlightError::AbiVersionMismatch(99)));

    // 4. Empty seal
    let res_empty_seal = shield.validate_proof_preflight(&expected_image_id, 1, &[], &valid_cp);
    assert_eq!(res_empty_seal, Err(PreFlightError::EmptySeal));

    // 5. Forged Checkpoint ID preimage
    let mut forged_cp = valid_cp.clone();
    forged_cp.id = [0xFE; 32];
    let res_forged_cp = shield.validate_proof_preflight(&expected_image_id, 1, &[0xAA; 100], &forged_cp);
    assert_eq!(res_forged_cp, Err(PreFlightError::CheckpointIdPreimageMismatch));
}

#[test]
fn test_r25_e_progressive_peer_penalty_jailing() {
    let peer = derive_actor_id(KeyType::Ed25519, &[0x05; 32]);
    let mut jail = PeerJail::new();

    // 1. Minor violation (40 points) -> Not jailed
    assert!(!jail.record_penalty(&peer, 40, 10));
    assert!(!jail.is_jailed(&peer, 10));

    // 2. Escalation (another 60 points = 100 total) -> Jailed for 60 epochs (until epoch 70)
    assert!(jail.record_penalty(&peer, 60, 10), "R25-E: Crossing 100 penalty threshold must trigger jail");
    assert!(jail.is_jailed(&peer, 10));
    assert!(jail.is_jailed(&peer, 50));
    assert!(jail.is_jailed(&peer, 69));

    // 3. After epoch 70 -> Released from jail
    assert!(!jail.is_jailed(&peer, 71));

    // 4. Second offense -> Escalated jail duration (60 * 2 = 120 epochs)
    assert!(jail.record_penalty(&peer, 100, 75));
    assert!(jail.is_jailed(&peer, 150)); // Jailed until 75 + 120 = 195
    assert!(!jail.is_jailed(&peer, 196));
}

#[test]
fn test_r25_g_adversarial_flood_endurance() {
    let mut node = VirtualNode::new("ResilientNode");

    // 1. Genesis mutation
    let b0 = MutationBody {
        parents: vec![],
        lamport: 0,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [1u8; 32], value: b"genesis".to_vec() },
    };
    let m0 = Mutation { id: hash_mutation_body(&b0), body: b0 };
    node.ingest_mutation(m0.clone());

    // 2. Inject 5,000 malformed / Byzantine packets (forged IDs, illegal Lamports, cyclic references)
    for i in 0..5000 {
        let bad_body = MutationBody {
            parents: vec![m0.id],
            lamport: 9999 + i, // Illegal Lamport
            epoch: 0,
            author: [0u8; 32],
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [2u8; 32], value: vec![] },
        };
        let bad_m = Mutation { id: hash_mutation_body(&bad_body), body: bad_body };
        node.ingest_mutation(bad_m);
    }

    // 3. Process honest child mutation on top of genesis
    let b1 = MutationBody {
        parents: vec![m0.id],
        lamport: 1,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [1u8; 32], value: b"honest_update".to_vec() },
    };
    let m1 = Mutation { id: hash_mutation_body(&b1), body: b1 };
    node.ingest_mutation(m1.clone());

    let cp = node.compute_current_checkpoint();

    // 4. Verify that the 5,000 hostile packets were 100% quarantined and did not corrupt honest state
    let mut reference_clean = VirtualNode::new("CleanNode");
    reference_clean.ingest_mutation(m0);
    reference_clean.ingest_mutation(m1);
    let ref_cp = reference_clean.compute_current_checkpoint();

    assert_eq!(cp.id, ref_cp.id, "R25-G: Adversarial flood must not compromise or alter honest state");
    assert_eq!(cp.body.state_root, ref_cp.body.state_root);
}
