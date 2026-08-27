use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::ObjectType;

#[test]
fn test_r71_25_a_mobile_capture_photo_commits_to_local_node() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp.path(), root_key);
    mobile_node.start().unwrap();

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

    let photo_payload = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46]; // JPEG header bytes
    let revoked = BTreeMap::new();

    let (obj_id, ns) = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile_node,
        &proof,
        "Summer Vacation Beach",
        photo_payload.clone(),
        10,
        &revoked,
        &root_actor,
    ).expect("Mobile capture failed");

    assert_eq!(ns, family_ns);
    assert_eq!(mobile_node.state.object_store.len(), 1);
    let obj = mobile_node.state.object_store.get(&obj_id).unwrap();
    assert_eq!(obj.object_type, ObjectType::PhotoMedia);
    assert_eq!(obj.payload_bytes, photo_payload);
}

#[test]
fn test_r71_25_b_mobile_create_document_commits_to_local_node() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp.path(), root_key);
    mobile_node.start().unwrap();

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

    let doc_payload = b"# Family Budget 2026\nIncome and expenses".to_vec();
    let revoked = BTreeMap::new();

    let (obj_id, ns) = SovereignProductSlice::mobile_create_family_document(
        &mut mobile_node,
        &proof,
        "Family_Budget.md",
        doc_payload.clone(),
        10,
        &revoked,
        &root_actor,
    ).expect("Mobile doc creation failed");

    assert_eq!(ns, family_ns);
    let obj = mobile_node.state.object_store.get(&obj_id).unwrap();
    assert_eq!(obj.object_type, ObjectType::DriveInode);
    assert_eq!(obj.payload_bytes, doc_payload);
}

#[test]
fn test_r71_25_c_multi_object_capture_in_family_space() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp.path(), root_key);
    mobile_node.start().unwrap();

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

    let revoked = BTreeMap::new();
    for i in 1..=4 {
        SovereignProductSlice::mobile_capture_family_photo(
            &mut mobile_node,
            &proof,
            &format!("Photo {}", i),
            format!("Binary photo data {}", i).into_bytes(),
            10,
            &revoked,
            &root_actor,
        ).unwrap();
    }

    assert_eq!(mobile_node.state.object_store.len(), 4);
}

#[test]
fn test_r71_25_d_empty_photo_payload_supported() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp.path(), root_key);
    mobile_node.start().unwrap();

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
        &mut mobile_node,
        &proof,
        "Empty",
        vec![],
        10,
        &BTreeMap::new(),
        &root_actor,
    ).unwrap();

    assert_eq!(mobile_node.state.object_store.get(&obj_id).unwrap().payload_bytes.len(), 0);
}

#[test]
fn test_r71_25_e_capture_records_latest_mutation_id() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp.path(), root_key);
    mobile_node.start().unwrap();

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

    SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile_node,
        &proof,
        "Mutated",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).unwrap();

    assert!(mobile_node.state.latest_mutation_id.is_some());
}

#[test]
fn test_r71_25_f_capture_preserves_space_tag_in_metadata() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x06u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp.path(), root_key);
    mobile_node.start().unwrap();

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
        &mut mobile_node,
        &proof,
        "Meta test",
        b"data".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).unwrap();

    let obj = mobile_node.state.object_store.get(&id).unwrap();
    assert_eq!(obj.metadata.get("space").unwrap(), "Family");
}
