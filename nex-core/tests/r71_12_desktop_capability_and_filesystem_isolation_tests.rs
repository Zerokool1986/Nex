use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::desktop::DesktopCapabilityGateway;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_READ, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::{ObjectType, NexObject};

#[test]
fn test_r71_12_a_valid_token_reads_desktop_object() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0x11; 32];
    let obj_id = [0x22; 32];
    let payload = b"Confidential Desktop Inode Payload";

    node.state.object_store.insert(obj_id, NexObject {
        object_id: obj_id,
        object_type: ObjectType::DriveInode,
        namespace: ns,
        owner_actor_id: root_actor,
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 1,
        winning_mutation_id: [0u8; 32],
        metadata: BTreeMap::new(),
        payload_bytes: payload.to_vec(),
        tombstoned: false,
    });

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
    let sig = root_key.sign(&token_hash).to_bytes();

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: sig.to_vec(),
    };

    let crl = BTreeMap::new();
    let read_res = DesktopCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        50,
        &crl,
    ).expect("Read failed");

    assert_eq!(read_res, payload);
}

#[test]
fn test_r71_12_b_valid_token_writes_desktop_object() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0x33; 32];
    let obj_id = [0x44; 32];
    let new_payload = b"New Desktop Note Written by Authorized Component";

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

    let crl = BTreeMap::new();
    let write_res = DesktopCapabilityGateway::authorize_and_write(
        &mut node,
        &root_actor,
        &ns,
        &obj_id,
        ObjectType::DriveInode,
        new_payload.to_vec(),
        &proof,
        50,
        &crl,
    );
    assert!(write_res.is_ok(), "Authorized write must succeed");

    let written = node.state.object_store.get(&obj_id).unwrap();
    assert_eq!(written.payload_bytes, new_payload);
}

#[test]
fn test_r71_12_c_insufficient_operation_mask_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0x55; 32];
    let obj_id = [0x66; 32];

    // Token only permits OP_READ, but component tries OP_WRITE
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
    let sig = root_key.sign(&token_hash).to_bytes();

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: sig.to_vec(),
    };

    let crl = BTreeMap::new();
    let res = DesktopCapabilityGateway::authorize_and_write(
        &mut node,
        &root_actor,
        &ns,
        &obj_id,
        ObjectType::DriveInode,
        b"unauthorized data".to_vec(),
        &proof,
        50,
        &crl,
    );
    assert!(res.is_err(), "Write with read-only token must be rejected");
}

#[test]
fn test_r71_12_d_wrong_namespace_isolation() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let permitted_ns = [0x77; 32];
    let target_ns = [0x88; 32];
    let obj_id = [0x99; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: permitted_ns,
        object_id: Some(obj_id),
        allowed_operations: OP_READ,
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

    let crl = BTreeMap::new();
    let res = DesktopCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &target_ns,
        &obj_id,
        &proof,
        50,
        &crl,
    );
    assert!(res.is_err(), "Namespace mismatch must be rejected");
}

#[test]
fn test_r71_12_e_expired_desktop_capability() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0xAA; 32];
    let obj_id = [0xBB; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_READ,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 30, // Expires at epoch 30
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

    let crl = BTreeMap::new();
    // Attempt at epoch 40 > 30
    let res = DesktopCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        40,
        &crl,
    );
    assert!(res.is_err(), "Expired token must be rejected");
}

#[test]
fn test_r71_12_f_zero_ambient_authority_default_denial() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x06u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key);
    node.start().unwrap();

    let ns = [0xCC; 32];
    let obj_id = [0xDD; 32];

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

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: vec![0xEEu8; 64], // invalid
    };

    let crl = BTreeMap::new();
    let res = DesktopCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        50,
        &crl,
    );
    assert!(res.is_err(), "Forged or ambient access must fail");
}
