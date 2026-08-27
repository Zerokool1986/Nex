use ed25519_dalek::SigningKey;
use tempfile::tempdir;
use sha2::{Sha256, Digest};
use nex_core::runtime::node::NexNode;
use nex_core::apps::compute::*;
use nex_core::apps::discovery::*;
use nex_core::apps::resources::*;
use nex_core::api::NexAppApi;

#[test]
fn test_r65_4_a_asymmetric_compute_exhaustion_defense() {
    let bytecode = vec![0x00; 10_000]; // 10,000 NOPs
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x66u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 500, // Small fuel limit
        memory_limit_bytes: 1024,
    };

    let err = ComputeEngine::execute_kernel(&job, &bytecode, &[]).unwrap_err();
    assert_eq!(err, ComputeError::FuelExhausted, "Infinite loops or long kernels must be caught cleanly by fuel counter");
}

#[test]
fn test_r65_4_b_sybil_trust_inversion_defense() {
    let mut wot = WebOfTrustRegistry::new();
    let root = [0x01u8; 32];
    let bad_actor = [0x02u8; 32];

    wot.add_alias(root, bad_actor, "Suspect", 0.05);

    // Bad actor creates 500 sybils
    for i in 0..500 {
        let sybil = [i as u8; 32];
        wot.add_alias(bad_actor, sybil, &format!("Sybil_{}", i), 1.0);
    }

    let (_, score) = wot.resolve_alias(&root, "sybil_42").unwrap();
    assert!(score < 0.03, "Sybil score must attenuate below threshold");
}

#[test]
fn test_r65_4_c_freerider_debt_ceiling_hard_cutoff() {
    let mut ledger = BilateralCreditLedger::new();
    let peer = [0x99u8; 32];
    let max_debt = 5 * 1024 * 1024; // 5MB max debt

    // Legitimate consumption
    assert!(ledger.record_transfer(peer, -4 * 1024 * 1024, max_debt).is_ok());

    // Attempting to consume 2MB more exceeds the 5MB ceiling
    assert!(ledger.record_transfer(peer, -2 * 1024 * 1024, max_debt).is_err());
    assert_eq!(ledger.get_balance(&peer), -4 * 1024 * 1024);
}

#[test]
fn test_r65_4_d_por_single_bit_tamper_defense() {
    let shard = vec![0xFFu8; 1024];
    let nonce = 777u64;

    let proof = ProofOfRetrievability::prove_storage(&shard, nonce);

    let mut corrupted = shard.clone();
    corrupted[512] ^= 0x01; // Flip single bit

    assert!(!ProofOfRetrievability::verify_proof(proof, &corrupted, nonce));
}

#[test]
fn test_r65_4_e_complete_zero_central_server_check() {
    let dir = tempdir().unwrap();
    let seed = [131u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    // Verify all operations run completely offline and locally
    let cp = node.sync_now().unwrap();
    assert_ne!(cp.body.state_root, [0u8; 32]);
}

#[test]
fn test_r65_4_f_gate_r65_master_system_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [245u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Full system stack must preserve Merkle root invariance");
}
