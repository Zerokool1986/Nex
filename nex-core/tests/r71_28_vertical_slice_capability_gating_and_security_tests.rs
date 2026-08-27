use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE, OP_READ};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r71_28_a_unauthorized_token_cannot_capture_to_family_space() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp.path(), root_key);
    mobile.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    // Token only grants OP_READ, but capture requires OP_WRITE
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_READ,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Unauthorized Photo",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    );

    assert!(res.is_err(), "Capture with OP_READ token must be rejected");
    assert_eq!(mobile.state.object_store.len(), 0);
}

#[test]
fn test_r71_28_b_expired_token_rejected_at_capture() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp.path(), root_key);
    mobile.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 5, // Expired at epoch 5
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    // Capture attempt at epoch 10 > 5
    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Expired Photo",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    );

    assert!(res.is_err(), "Expired capability proof must fail");
    assert_eq!(mobile.state.object_store.len(), 0);
}

#[test]
fn test_r71_28_c_revoked_token_rejected_at_capture() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp.path(), root_key);
    mobile.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let mut revoked = BTreeMap::new();
    revoked.insert(token_hash, 5); // Revoked at epoch 5

    // Capture attempt at epoch 10 > 5
    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Revoked Photo",
        b"data".to_vec(),
        10,
        &revoked,
        &root_actor,
    );

    assert!(res.is_err(), "Revoked capability proof must fail");
    assert_eq!(mobile.state.object_store.len(), 0);
}

#[test]
fn test_r71_28_d_forged_signature_rejected_at_capture() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp.path(), root_key);
    mobile.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: vec![0xDE, 0xAD, 0xBE, 0xEF], // forged signature
    };

    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Forged Photo",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    );

    assert!(res.is_err(), "Forged signature must be rejected");
    assert_eq!(mobile.state.object_store.len(), 0);
}

#[test]
fn test_r71_28_e_space_mismatch_rejected_at_capture() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp.path(), root_key);
    mobile.start().unwrap();

    let personal_ns = NexHomeShell::space_to_namespace(SpaceType::Personal);
    // Token is valid for Personal Space, but capture is attempting Family Space write
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: personal_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Wrong Space Photo",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    );

    assert!(res.is_err(), "Namespace/Space mismatch must be rejected");
    assert_eq!(mobile.state.object_store.len(), 0);
}

#[test]
fn test_r71_28_f_valid_flow_preserves_strict_capability_proof() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x06u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp.path(), root_key);
    mobile.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (obj_id, _) = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Valid Photo",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).expect("Valid capture must succeed");

    assert_eq!(mobile.state.object_store.len(), 1);
    assert!(mobile.state.object_store.contains_key(&obj_id));
}
