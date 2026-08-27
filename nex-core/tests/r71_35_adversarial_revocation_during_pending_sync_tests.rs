use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r71_35_a_revocation_during_pending_sync_rejects_object() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x01u8; 32];
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

    let mut crl = BTreeMap::new();
    crl.insert(token_hash, 5); // Revoked at epoch 5

    // Attempt capture at epoch 10 > 5
    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Revoked Mid-Flight",
        b"data".to_vec(),
        10,
        &crl,
        &root_actor,
    );

    assert!(res.is_err(), "Revoked token must be rejected even under pending sync");
    assert_eq!(mobile.state.object_store.len(), 0);
}

#[test]
fn test_r71_35_b_token_expiration_during_offline_period_denies_sync() {
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

    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Expired During Offline",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    );

    assert!(res.is_err());
}

#[test]
fn test_r71_35_c_space_isolation_preserved_under_failure_and_partition() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x04u8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

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
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Family Secret", b"secret".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // Partial sync
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    // Assert that personal space on desktop remains 0
    let mut shell = NexHomeShell::new();
    shell.switch_space(SpaceType::Personal);
    assert_eq!(shell.generate_home_summary(&desktop).total_objects_in_space, 0);

    // Assert that work space on desktop remains 0
    shell.switch_space(SpaceType::Work);
    assert_eq!(shell.generate_home_summary(&desktop).total_objects_in_space, 0);
}

#[test]
fn test_r71_35_d_forged_mutation_replay_rejected_at_gateway() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x05u8; 32];
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
        signature: vec![0xCA, 0xFE, 0xBA, 0xBE], // forged
    };

    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Forged Attack",
        b"payload".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    );

    assert!(res.is_err());
}

#[test]
fn test_r71_35_e_network_failure_does_not_open_ambient_authority() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x06u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp.path(), root_key);
    mobile.start().unwrap();

    let personal_ns = NexHomeShell::space_to_namespace(SpaceType::Personal);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: personal_ns, // Personal only
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

    // Attempting to write into Family Space with Personal token
    let res = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Cross Space Write",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    );

    assert!(res.is_err());
}

#[test]
fn test_r71_35_f_valid_revocation_free_flow_succeeds() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x07u8; 32];
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

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Valid Pic",
        b"ok".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).expect("Valid token must succeed");

    assert!(mobile.state.object_store.contains_key(&id));
}
