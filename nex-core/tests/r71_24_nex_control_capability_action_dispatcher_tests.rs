use std::collections::{BTreeMap, BTreeSet};
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::dispatcher::UiActionDispatcher;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE, OP_READ};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::ObjectType;

#[test]
fn test_r71_24_a_authorized_ui_create_object() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pk = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pk);

    let mut node = NexNode::new(tmp.path(), root_key);
    node.start().unwrap();

    let ns = [0x11; 32];
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pk.to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let revoked = BTreeMap::new();
    let res = UiActionDispatcher::dispatch_ui_create_object(
        &mut node,
        &proof,
        ns,
        ObjectType::PhotoMedia,
        BTreeMap::new(),
        b"Photo payload".to_vec(),
        50,
        &revoked,
        &root_actor,
    );

    assert!(res.is_ok(), "Authorized UI create must succeed");
    assert_eq!(node.state.object_store.len(), 1);
}

#[test]
fn test_r71_24_b_unauthorized_ui_create_rejected() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pk = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pk);

    let mut node = NexNode::new(tmp.path(), root_key);
    node.start().unwrap();

    let ns = [0x22; 32];
    // Token only grants OP_READ
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: None,
        allowed_operations: OP_READ,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pk.to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let revoked = BTreeMap::new();
    let res = UiActionDispatcher::dispatch_ui_create_object(
        &mut node,
        &proof,
        ns,
        ObjectType::PhotoMedia,
        BTreeMap::new(),
        b"Payload".to_vec(),
        50,
        &revoked,
        &root_actor,
    );

    assert!(res.is_err(), "UI create with OP_READ token must be rejected");
    assert_eq!(node.state.object_store.len(), 0);
}

#[test]
fn test_r71_24_c_expired_token_ui_create_rejected() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pk = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pk);

    let mut node = NexNode::new(tmp.path(), root_key);
    node.start().unwrap();

    let ns = [0x33; 32];
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 40, // Expires at epoch 40
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pk.to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let revoked = BTreeMap::new();
    // Dispatch at epoch 50 > 40
    let res = UiActionDispatcher::dispatch_ui_create_object(
        &mut node,
        &proof,
        ns,
        ObjectType::PhotoMedia,
        BTreeMap::new(),
        b"Payload".to_vec(),
        50,
        &revoked,
        &root_actor,
    );

    assert!(res.is_err(), "Expired capability token must be rejected in UI dispatcher");
}

#[test]
fn test_r71_24_d_ui_revoke_device_updates_crl() {
    let mut crl = BTreeSet::new();
    let dev_actor = [0x99; 32];

    assert_eq!(UiActionDispatcher::dispatch_ui_revoke_device(&mut crl, dev_actor), 1);
    assert!(crl.contains(&dev_actor));
}

#[test]
fn test_r71_24_e_revoked_token_ui_create_rejected() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pk = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pk);

    let mut node = NexNode::new(tmp.path(), root_key);
    node.start().unwrap();

    let ns = [0x44; 32];
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pk.to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let mut revoked = BTreeMap::new();
    revoked.insert(token_hash, 25); // Revoked at epoch 25

    // Attempt create at epoch 50 > 25
    let res = UiActionDispatcher::dispatch_ui_create_object(
        &mut node,
        &proof,
        ns,
        ObjectType::PhotoMedia,
        BTreeMap::new(),
        b"Payload".to_vec(),
        50,
        &revoked,
        &root_actor,
    );

    assert!(res.is_err(), "Revoked token must be rejected in UI dispatcher");
}

#[test]
fn test_r71_24_f_forged_signature_ui_create_rejected() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pk = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pk);

    let mut node = NexNode::new(tmp.path(), root_key);
    node.start().unwrap();

    let ns = [0x55; 32];
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pk.to_vec()),
        parent_proof: None,
        signature: vec![0xEE; 64], // invalid forged signature
    };

    let revoked = BTreeMap::new();
    let res = UiActionDispatcher::dispatch_ui_create_object(
        &mut node,
        &proof,
        ns,
        ObjectType::PhotoMedia,
        BTreeMap::new(),
        b"Payload".to_vec(),
        50,
        &revoked,
        &root_actor,
    );

    assert!(res.is_err(), "Forged signature must be rejected in UI dispatcher");
}
