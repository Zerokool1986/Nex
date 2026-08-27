use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::mobile::AndroidCapabilityGateway;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_READ, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::{ObjectType, NexObject};

#[test]
fn test_r71_8_a_valid_capability_token_reads_object() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0x11; 32];
    let obj_id = [0x22; 32];
    let payload = b"Sovereign encrypted object payload";

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

    // Create capability token
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
    let read_res = AndroidCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        50,
        &crl,
    ).expect("Capability authorization failed");

    assert_eq!(read_res, payload);
}

#[test]
fn test_r71_8_b_unauthorized_capability_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key);
    node.start().unwrap();

    let ns = [0x33; 32];
    let obj_id = [0x44; 32];

    // Token only permits OP_WRITE, but client attempts OP_READ
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_WRITE, // insufficient for read!
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        parent_proof: None,
        signature: vec![0u8; 64],
    };

    let crl = BTreeMap::new();
    let res = AndroidCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        50,
        &crl,
    );

    assert!(res.is_err(), "Insufficient operations mask must be rejected");
}

#[test]
fn test_r71_8_c_expired_capability_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0x55; 32];
    let obj_id = [0x66; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id),
        allowed_operations: OP_READ,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 40, // Expires at epoch 40
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
    // Attempt read at epoch 50 > 40
    let res = AndroidCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        50,
        &crl,
    );
    assert!(res.is_err(), "Expired capability proof must fail");
}

#[test]
fn test_r71_8_d_wrong_object_capability_mismatch() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0x77; 32];
    let obj_id_permitted = [0x88; 32];
    let obj_id_target = [0x99; 32];

    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: ns,
        object_id: Some(obj_id_permitted),
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
    let res = AndroidCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id_target,
        &proof,
        50,
        &crl,
    );
    assert!(res.is_err(), "Capability for obj_permitted cannot read obj_target");
}

#[test]
fn test_r71_8_e_revoked_capability_epoch_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key.clone());
    node.start().unwrap();

    let ns = [0x0A; 32];
    let obj_id = [0x0B; 32];

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

    let mut crl = BTreeMap::new();
    crl.insert(token_hash, 15u64); // Revoked at epoch 15

    // Attempt access at epoch 20 > 15
    let res = AndroidCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        20,
        &crl,
    );
    assert!(res.is_err(), "Revoked capability epoch must fail");
}

#[test]
fn test_r71_8_f_zero_ambient_authority_default_denial() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let root_seed = [0x06u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let mut node = NexNode::new(temp_dir.path().to_path_buf(), root_key);
    node.start().unwrap();

    let ns = [0x0C; 32];
    let obj_id = [0x0D; 32];

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
        signature: vec![0xEEu8; 64], // invalid signature
    };

    let crl = BTreeMap::new();
    let res = AndroidCapabilityGateway::authorize_and_read(
        &node,
        &root_actor,
        &ns,
        &obj_id,
        &proof,
        50,
        &crl,
    );
    assert!(res.is_err(), "Forged or ambient access must be denied");
}
