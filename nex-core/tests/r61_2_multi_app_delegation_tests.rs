use ed25519_dalek::SigningKey;
use nex_core::apps::platform::GroupFederationEngine;
use nex_core::identity::types::{KeyType, OP_READ, OP_WRITE, OP_ALL};
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r61_2_a_group_root_capability_drive() {
    let seed = [151u8; 32];
    let group_root = SigningKey::from_bytes(&seed);
    let member = [0x55u8; 32];
    let group_id = [0x11u8; 32];

    let proof = GroupFederationEngine::create_group_capability_token(
        &group_root,
        member,
        group_id,
        OP_READ | OP_WRITE,
    );

    let expected_issuer = derive_actor_id(KeyType::Ed25519, &group_root.verifying_key().to_bytes());
    assert_eq!(proof.token.issuer, expected_issuer);
    assert_eq!(proof.token.subject, member);
    assert_eq!(proof.token.allowed_operations, OP_READ | OP_WRITE);
}

#[test]
fn test_r61_2_b_multi_namespace_delegation() {
    let seed = [152u8; 32];
    let group_root = SigningKey::from_bytes(&seed);
    let member = [0x66u8; 32];

    let groups = [
        [0x11u8; 32], // Drive Group
        [0x22u8; 32], // Photos Group
        [0x33u8; 32], // Chat Group
        [0x44u8; 32], // Maps Group
    ];

    for g in groups {
        let proof = GroupFederationEngine::create_group_capability_token(
            &group_root,
            member,
            g,
            OP_ALL,
        );
        assert_eq!(proof.token.allowed_operations, OP_ALL);
    }
}

#[test]
fn test_r61_2_c_read_only_guest_delegation() {
    let seed = [153u8; 32];
    let group_root = SigningKey::from_bytes(&seed);
    let guest = [0x77u8; 32];
    let group_id = [0x22u8; 32];

    let proof = GroupFederationEngine::create_group_capability_token(
        &group_root,
        guest,
        group_id,
        OP_READ,
    );

    assert_eq!(proof.token.allowed_operations, OP_READ);
    assert_eq!(proof.token.allowed_operations & OP_WRITE, 0);
}

#[test]
fn test_r61_2_d_token_expiration() {
    let seed = [154u8; 32];
    let group_root = SigningKey::from_bytes(&seed);
    let member = [0x88u8; 32];
    let group_id = [0x11u8; 32];

    let proof = GroupFederationEngine::create_group_capability_token(
        &group_root,
        member,
        group_id,
        OP_READ,
    );

    assert!(proof.token.expires_at_epoch > 0);
}

#[test]
fn test_r61_2_e_distinct_issuer_keys() {
    let seed1 = [155u8; 32];
    let seed2 = [156u8; 32];
    let group1 = SigningKey::from_bytes(&seed1);
    let group2 = SigningKey::from_bytes(&seed2);

    let member = [0x99u8; 32];
    let group_id = [0x11u8; 32];

    let proof1 = GroupFederationEngine::create_group_capability_token(&group1, member, group_id, OP_READ);
    let proof2 = GroupFederationEngine::create_group_capability_token(&group2, member, group_id, OP_READ);

    assert_ne!(proof1.token.issuer, proof2.token.issuer);
    assert_ne!(proof1.signature, proof2.signature);
}

#[test]
fn test_r61_2_f_zero_regression_delegation_lifecycle() {
    let seed = [157u8; 32];
    let group_root = SigningKey::from_bytes(&seed);
    for i in 0..10 {
        let member = [i + 50; 32];
        let proof = GroupFederationEngine::create_group_capability_token(&group_root, member, [0xAA; 32], OP_READ);
        assert_eq!(proof.token.subject, member);
    }
}
