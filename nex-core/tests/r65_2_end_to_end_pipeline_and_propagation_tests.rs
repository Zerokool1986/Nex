use sha2::{Sha256, Digest};
use nex_core::apps::groups::*;
use nex_core::apps::resources::*;
use nex_core::apps::compute::*;
use nex_core::apps::discovery::*;
use nex_core::apps::platform::*;

#[test]
fn test_r65_2_a_full_ecosystem_pipeline_execution() {
    // 1. Create Group
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Core Family", admin);
    let child = [0x02u8; 32];
    group.add_member(child, GroupRole::Member);
    assert_eq!(group.epoch, 1);

    // 2. Allocate Family Storage Pool
    let mut pool = FamilyStoragePool::new(100 * 1024 * 1024);
    pool.set_member_limit(child, 20 * 1024 * 1024);
    assert!(pool.allocate_storage(&child, 5 * 1024 * 1024).is_ok());

    // 3. Shard Data into Resource Grid
    let secret_document = b"Sovereign family document content.".to_vec();
    let shards = ErasureCoder::split(&secret_document, 4);

    // 4. Verify PoR Challenge for Shard 0
    let shard_proof = ProofOfRetrievability::prove_storage(&shards[0].data, 100);
    assert!(ProofOfRetrievability::verify_proof(shard_proof, &shards[0].data, 100));

    // 5. Reconstruct from Shards
    let reconstructed = ErasureCoder::reconstruct(&shards[0..4], 4, secret_document.len()).unwrap();
    assert_eq!(reconstructed, secret_document);

    // 6. Run Compute Kernel over Reconstructed Payload
    let bytecode = vec![0x01]; // Identity transform
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x88u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 1000,
        memory_limit_bytes: 1024 * 1024,
    };

    let compute_res = ComputeEngine::execute_kernel(&job, &bytecode, &[reconstructed]).unwrap();
    assert_eq!(compute_res.output_bytes, secret_document);
}

#[test]
fn test_r65_2_b_group_ratchet_forward_secrecy_propagation() {
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Secret Guild", admin);
    let traitor = [0x99u8; 32];
    group.add_member(traitor, GroupRole::Member);

    let epoch1_secret = group.epoch_secret;
    assert_eq!(group.epoch, 1);
    assert!(group.is_active_member(&traitor));

    // Evict traitor
    assert!(group.remove_member(&traitor).is_ok());

    // Verify epoch increment and irreversible secret ratchet
    assert_eq!(group.epoch, 2);
    assert_ne!(group.epoch_secret, epoch1_secret);
    assert!(!group.is_active_member(&traitor));
}

#[test]
fn test_r65_2_c_web_of_trust_to_petname_pipeline() {
    let mut wot = WebOfTrustRegistry::new();
    let root = [0x01u8; 32];
    let device_a = [0x02u8; 32];
    let device_b = [0x03u8; 32];

    wot.add_alias(root, device_a, "Laptop", 0.9);
    wot.add_alias(device_a, device_b, "Server", 0.8);

    let (target, score) = wot.resolve_alias(&root, "server").unwrap();
    assert_eq!(target, device_b);
    assert!((score - 0.36).abs() < 1e-6); // 0.9 * 0.8 * 0.5 = 0.36
}

#[test]
fn test_r65_2_d_uri_resolution_to_app_routing() {
    let actor_hex = hex::encode([0xAAu8; 32]);
    let ns_hex = hex::encode([0xBBu8; 32]);
    let uri = format!("nex://{}/{}/chat/channel_alpha", actor_hex, ns_hex);
    let parsed = NexUri::parse(&uri).unwrap();

    assert_eq!(parsed.raw, uri);
    assert_eq!(parsed.actor_id, [0xAAu8; 32]);
    assert_eq!(parsed.namespace, [0xBBu8; 32]);
    assert_eq!(parsed.path, "/chat/channel_alpha");
}

#[test]
fn test_r65_2_e_bilateral_credit_relay_conservation() {
    let mut ledger = BilateralCreditLedger::new();
    let node_a = [0x01u8; 32];
    let node_b = [0x02u8; 32];

    // Node A provides 50MB to Node B
    ledger.record_transfer(node_a, 50 * 1024 * 1024, 100 * 1024 * 1024).unwrap();
    // Node B consumes 20MB from Node A
    ledger.record_transfer(node_b, -20 * 1024 * 1024, 100 * 1024 * 1024).unwrap();

    assert_eq!(ledger.get_balance(&node_a), 50 * 1024 * 1024);
    assert_eq!(ledger.get_balance(&node_b), -20 * 1024 * 1024);
}

#[test]
fn test_r65_2_f_zero_regression_pipeline_lifecycle() {
    let mut search = InvertedSearchIndex::new();
    let doc_id = [0x11u8; 32];
    search.index_document(doc_id, "End-to-end integration verified");
    assert_eq!(search.search("integration"), vec![doc_id]);
}
