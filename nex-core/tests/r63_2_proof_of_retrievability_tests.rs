use nex_core::apps::resources::*;

#[test]
fn test_r63_2_a_por_proof_generation_and_verification() {
    let shard_data = b"Encrypted shard content stored at provider node.".to_vec();
    let nonce = 123456789u64;

    let proof = ProofOfRetrievability::prove_storage(&shard_data, nonce);
    assert!(ProofOfRetrievability::verify_proof(proof, &shard_data, nonce));
}

#[test]
fn test_r63_2_b_tampered_shard_rejected() {
    let shard_data = b"Valid uncorrupted shard payload.".to_vec();
    let nonce = 987654321u64;

    let proof = ProofOfRetrievability::prove_storage(&shard_data, nonce);

    let mut tampered = shard_data.clone();
    tampered[0] ^= 0x01; // Bit flip

    assert!(!ProofOfRetrievability::verify_proof(proof, &tampered, nonce));
}

#[test]
fn test_r63_2_c_wrong_nonce_rejected() {
    let shard_data = b"Constant shard payload.".to_vec();
    let nonce = 100u64;

    let proof = ProofOfRetrievability::prove_storage(&shard_data, nonce);
    assert!(!ProofOfRetrievability::verify_proof(proof, &shard_data, nonce + 1));
}

#[test]
fn test_r63_2_d_deterministic_proof_output() {
    let shard_data = b"Deterministic payload".to_vec();
    let nonce = 42u64;

    let p1 = ProofOfRetrievability::prove_storage(&shard_data, nonce);
    let p2 = ProofOfRetrievability::prove_storage(&shard_data, nonce);
    assert_eq!(p1, p2);
}

#[test]
fn test_r63_2_e_empty_shard_por() {
    let shard_data = vec![];
    let nonce = 1u64;

    let proof = ProofOfRetrievability::prove_storage(&shard_data, nonce);
    assert!(ProofOfRetrievability::verify_proof(proof, &shard_data, nonce));
}

#[test]
fn test_r63_2_f_zero_regression_por_lifecycle() {
    for i in 0..10 {
        let shard = vec![i as u8; 64];
        let proof = ProofOfRetrievability::prove_storage(&shard, i as u64);
        assert!(ProofOfRetrievability::verify_proof(proof, &shard, i as u64));
    }
}
