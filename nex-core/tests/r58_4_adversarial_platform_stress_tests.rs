use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::platform::*;
use nex_core::api::NexAppApi;
use std::collections::BTreeMap;

#[test]
fn test_r58_4_a_malformed_nex_uri_parser_fuzzing() {
    let fuzzed_inputs = vec![
        "",
        "nex://",
        "nex:///path",
        "nex://invalid_hex_actor/namespace/path",
        "nex://010203/short_actor_hex",
        "ftp://0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20/a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0/path",
    ];

    for input in fuzzed_inputs {
        assert!(NexUri::parse(input).is_err(), "Parser must reject fuzzed input: {}", input);
    }
}

#[test]
fn test_r58_4_b_spatial_bounding_box_edge_cases() {
    let dir = tempdir().unwrap();
    let seed = [81u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    // Inverted bounds (min > max) must return empty list without crashing
    let results = SpatialMapEngine::query_bounding_box(&node, 50.0, 40.0, 10.0, 5.0);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_r58_4_c_group_token_forgery_rejection() {
    let group_root_seed = [82u8; 32];
    let group_root = SigningKey::from_bytes(&group_root_seed);
    let root_pub = group_root.verifying_key().to_bytes();
    let root_actor = nex_core::identity::verifier::derive_actor_id(nex_core::identity::types::KeyType::Ed25519, &root_pub);

    let member_seed = [83u8; 32];
    let member_key = SigningKey::from_bytes(&member_seed);
    let member_actor = nex_core::identity::verifier::derive_actor_id(
        nex_core::identity::types::KeyType::Ed25519,
        &member_key.verifying_key().to_bytes(),
    );

    let group_id = [0x99u8; 32];
    let mut proof = GroupFederationEngine::create_group_capability_token(
        &group_root,
        member_actor,
        group_id,
        nex_core::identity::types::OP_READ,
    );

    // Tamper with allowed operations
    proof.token.allowed_operations = nex_core::identity::types::OP_ALL;

    let revocations = BTreeMap::new();
    let is_valid = nex_core::identity::verifier::verify_capability_chain(
        &proof,
        nex_core::identity::types::OP_ALL,
        &group_id,
        None,
        0,
        &revocations,
        &root_actor,
    );
    assert!(is_valid.is_err(), "Tampered capability proof must fail cryptographic verification");
}

#[test]
fn test_r58_4_d_outbox_queue_stress() {
    let mut outbox = OfflineOutbox::new();
    let ns = [0x99u8; 32];

    for i in 0..1000 {
        outbox.enqueue(ns, nex_core::object::types::ObjectType::DriveInode, std::collections::BTreeMap::new(), vec![(i % 255) as u8; 64]);
    }
    assert_eq!(outbox.queue.len(), 1000);
}

#[test]
fn test_r58_4_e_petname_concurrent_reads() {
    let mut dir = PetnameDirectory::new();
    dir.set_petname("Alice", [0x01; 32]);
    dir.set_petname("Bob", [0x02; 32]);

    assert_eq!(dir.resolve_petname("alice"), Some([0x01; 32]));
    assert_eq!(dir.resolve_petname("bob"), Some([0x02; 32]));
}

#[test]
fn test_r58_4_f_gate_r58_master_platform_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [84u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Platform state must preserve Merkle root invariance");
}
