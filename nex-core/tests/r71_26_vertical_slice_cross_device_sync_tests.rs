use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r71_26_a_sync_mobile_photo_to_desktop() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop_node = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x02u8; 32]));
    mobile_node.start().unwrap();
    desktop_node.start().unwrap();

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

    let (obj_id, _) = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile_node,
        &proof,
        "Vacation sunset",
        b"Binary JPEG image bytes".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).unwrap();

    assert_eq!(mobile_node.state.object_store.len(), 1);
    assert_eq!(desktop_node.state.object_store.len(), 0);

    let synced_batches = SovereignProductSlice::sync_mobile_to_desktop(&mut mobile_node, &mut desktop_node);
    assert!(synced_batches > 0);

    assert_eq!(desktop_node.state.object_store.len(), 1);
    let desktop_obj = desktop_node.state.object_store.get(&obj_id).unwrap();
    assert_eq!(desktop_obj.payload_bytes, b"Binary JPEG image bytes");
}

#[test]
fn test_r71_26_b_sync_multiple_photos_and_documents() {
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

    // Capture 3 photos and 2 docs
    for i in 1..=3 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("P{}", i), vec![i as u8; 50], 10, &BTreeMap::new(), &root_actor).unwrap();
    }
    for i in 1..=2 {
        SovereignProductSlice::mobile_create_family_document(&mut mobile, &proof, &format!("D{}.md", i), vec![i as u8; 20], 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    assert_eq!(mobile.state.object_store.len(), 5);

    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(desktop.state.object_store.len(), 5);
}

#[test]
fn test_r71_26_c_sync_idempotency_repeated_rounds() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x06u8; 32]));
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

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Test", b"xyz".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // First sync
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(desktop.state.object_store.len(), 1);

    // Repeat 5 times
    for _ in 0..5 {
        SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    }
    assert_eq!(desktop.state.object_store.len(), 1);
}

#[test]
fn test_r71_26_d_empty_mobile_sync_does_not_mutate_desktop() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&[0x07u8; 32]));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x08u8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    assert_eq!(SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop), 0);
    assert_eq!(desktop.state.object_store.len(), 0);
}

#[test]
fn test_r71_26_e_bidirectional_sync_reconciles_both_hosts() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x09u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x0Au8; 32]));
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

    // Mobile captures photo
    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Mobile Pic", b"m_data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // Desktop captures doc
    SovereignProductSlice::mobile_create_family_document(&mut desktop, &proof, "Desktop Doc", b"d_data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    assert_eq!(mobile.state.object_store.len(), 1);
    assert_eq!(desktop.state.object_store.len(), 1);

    // Sync mobile -> desktop, then desktop -> mobile
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    SovereignProductSlice::sync_mobile_to_desktop(&mut desktop, &mut mobile);

    assert_eq!(mobile.state.object_store.len(), 2);
    assert_eq!(desktop.state.object_store.len(), 2);
}

#[test]
fn test_r71_26_f_large_payload_sync_integrity() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x0Bu8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x0Cu8; 32]));
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

    let large_data = vec![0x42u8; 100_000]; // 100 KB payload
    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "HighRes", large_data.clone(), 10, &BTreeMap::new(), &root_actor).unwrap();

    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    let synced = desktop.state.object_store.get(&id).unwrap();
    assert_eq!(synced.payload_bytes.len(), 100_000);
    assert_eq!(synced.payload_bytes, large_data);
}
