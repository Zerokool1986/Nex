use std::collections::{BTreeMap, BTreeSet};
use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use nex_core::sync::gateway::SyncCapabilityGateway;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, DeviceCertificate, KeyType, OP_WRITE, OP_READ};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token, DOMAIN_DEVICE_CERT};

#[test]
fn test_r71_20_a_valid_sync_ingest_authorization() {
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let ns = [0x11; 32];
    let obj_id = [0x22; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    let token_hash = hash_capability_token(&token);
    let sig = root_key.sign(&token_hash).to_bytes();

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: sig.to_vec(),
    };

    let revoked_devices = BTreeSet::new();
    let revoked_tokens = BTreeMap::new();

    let res = SyncCapabilityGateway::verify_sync_ingest(
        None,
        &proof,
        &ns,
        &obj_id,
        50,
        &revoked_devices,
        &revoked_tokens,
        &root_actor,
    );

    assert!(res.is_ok(), "Valid sync capability must authorize write");
}

#[test]
fn test_r71_20_b_revoked_device_certificate_rejection() {
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_key = SigningKey::from_bytes(&[0x03u8; 32]);
    let device_pk = device_key.verifying_key().to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pk);

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&device_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&500u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let sig = root_key.sign(&cert_hash).to_bytes();

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: sig.to_vec(),
    };

    let ns = [0x33; 32];
    let obj_id = [0x44; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    // Device is revoked on CRL
    let mut revoked_devices = BTreeSet::new();
    revoked_devices.insert(device_actor);
    let revoked_tokens = BTreeMap::new();

    let res = SyncCapabilityGateway::verify_sync_ingest(
        Some(&cert),
        &proof,
        &ns,
        &obj_id,
        250,
        &revoked_devices,
        &revoked_tokens,
        &root_actor,
    );

    assert!(res.is_err(), "Revoked device key must be rejected during sync");
}

#[test]
fn test_r71_20_c_insufficient_operation_mask_rejection() {
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let ns = [0x55; 32];
    let obj_id = [0x66; 32];

    // Token only permits OP_READ, but sync ingest requires OP_WRITE
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_READ,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let revoked_devices = BTreeSet::new();
    let revoked_tokens = BTreeMap::new();

    let res = SyncCapabilityGateway::verify_sync_ingest(
        None,
        &proof,
        &ns,
        &obj_id,
        50,
        &revoked_devices,
        &revoked_tokens,
        &root_actor,
    );

    assert!(res.is_err(), "Sync write with read-only token must be rejected");
}

#[test]
fn test_r71_20_d_wrong_namespace_rejection() {
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let permitted_ns = [0x77; 32];
    let target_ns = [0x88; 32];
    let obj_id = [0x99; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: permitted_ns,
        object_id: Some(obj_id),
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let revoked_devices = BTreeSet::new();
    let revoked_tokens = BTreeMap::new();

    let res = SyncCapabilityGateway::verify_sync_ingest(
        None,
        &proof,
        &target_ns,
        &obj_id,
        50,
        &revoked_devices,
        &revoked_tokens,
        &root_actor,
    );

    assert!(res.is_err(), "Namespace mismatch must be rejected");
}

#[test]
fn test_r71_20_e_expired_capability_epoch_rejection() {
    let root_seed = [0x06u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let ns = [0xAA; 32];
    let obj_id = [0xBB; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 40, // Expires at epoch 40
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let revoked_devices = BTreeSet::new();
    let revoked_tokens = BTreeMap::new();

    // Attempt ingest at epoch 50 > 40
    let res = SyncCapabilityGateway::verify_sync_ingest(
        None,
        &proof,
        &ns,
        &obj_id,
        50,
        &revoked_devices,
        &revoked_tokens,
        &root_actor,
    );

    assert!(res.is_err(), "Expired capability proof must fail during sync");
}

#[test]
fn test_r71_20_f_forged_capability_signature_default_denial() {
    let root_seed = [0x07u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let ns = [0xCC; 32];
    let obj_id = [0xDD; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: vec![0xEEu8; 64], // invalid forged signature
    };

    let revoked_devices = BTreeSet::new();
    let revoked_tokens = BTreeMap::new();

    let res = SyncCapabilityGateway::verify_sync_ingest(
        None,
        &proof,
        &ns,
        &obj_id,
        50,
        &revoked_devices,
        &revoked_tokens,
        &root_actor,
    );

    assert!(res.is_err(), "Forged sync signature must be rejected");
}
