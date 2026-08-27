use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::panels::ContextualPanelsEngine;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::api::NexAppApi;

#[test]
fn test_r71_27_a_desktop_presents_family_space_synced_photos_and_docs() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x02u8; 32]));
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

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Beach", b"photo_bytes".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::mobile_create_family_document(&mut mobile, &proof, "Budget.md", b"doc_bytes".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // Sync to Desktop
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    let (count, titles, status) = SovereignProductSlice::desktop_present_family_space(&desktop);
    assert_eq!(count, 2);
    assert_eq!(titles.len(), 2);
    assert_eq!(status, "All up to date");
}

#[test]
fn test_r71_27_b_personal_space_isolated_from_family_space() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x04u8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    let personal_ns = NexHomeShell::space_to_namespace(SpaceType::Personal);
    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);

    let token_fam = CapabilityProof {
        token: CapabilityToken {
            issuer: root_actor,
            subject: root_actor,
            namespace: family_ns,
            object_id: None,
            allowed_operations: OP_WRITE,
            delegation_depth: 0,
            not_before_epoch: 1,
            expires_at_epoch: 100,
            parent_token_hash: None,
        },
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&hash_capability_token(&CapabilityToken {
            issuer: root_actor,
            subject: root_actor,
            namespace: family_ns,
            object_id: None,
            allowed_operations: OP_WRITE,
            delegation_depth: 0,
            not_before_epoch: 1,
            expires_at_epoch: 100,
            parent_token_hash: None,
        })).to_bytes().to_vec(),
    };

    let token_per = CapabilityProof {
        token: CapabilityToken {
            issuer: root_actor,
            subject: root_actor,
            namespace: personal_ns,
            object_id: None,
            allowed_operations: OP_WRITE,
            delegation_depth: 0,
            not_before_epoch: 1,
            expires_at_epoch: 100,
            parent_token_hash: None,
        },
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&hash_capability_token(&CapabilityToken {
            issuer: root_actor,
            subject: root_actor,
            namespace: personal_ns,
            object_id: None,
            allowed_operations: OP_WRITE,
            delegation_depth: 0,
            not_before_epoch: 1,
            expires_at_epoch: 100,
            parent_token_hash: None,
        })).to_bytes().to_vec(),
    };

    // 1 Family photo
    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &token_fam, "Family Picnic", b"fam".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // 1 Personal doc
    nex_core::runtime::dispatcher::UiActionDispatcher::dispatch_ui_create_object(
        &mut mobile,
        &token_per,
        personal_ns,
        nex_core::object::types::ObjectType::DriveInode,
        BTreeMap::new(),
        b"personal".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).unwrap();

    assert_eq!(mobile.state.object_store.len(), 2);
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    // Desktop Family Space shows ONLY 1 object
    let (fam_count, _, _) = SovereignProductSlice::desktop_present_family_space(&desktop);
    assert_eq!(fam_count, 1);

    // Switching desktop shell to Personal Space shows the other 1 object
    let mut shell = NexHomeShell::new();
    shell.switch_space(SpaceType::Personal);
    assert_eq!(shell.generate_home_summary(&desktop).total_objects_in_space, 1);
}

#[test]
fn test_r71_27_c_contextual_storage_panel_breakdown_after_sync() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Photo", vec![0xAA; 500], 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::mobile_create_family_document(&mut mobile, &proof, "Doc", vec![0xBB; 300], 10, &BTreeMap::new(), &root_actor).unwrap();

    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    let storage_panel = ContextualPanelsEngine::project_storage_panel(&desktop);
    assert_eq!(storage_panel.total_used_bytes, 800);
    assert_eq!(storage_panel.photos_bytes, 500);
    assert_eq!(storage_panel.drive_bytes, 300);
    assert_eq!(storage_panel.objects_count, 2);
}

#[test]
fn test_r71_27_d_deletion_propagates_and_disappears_from_family_space() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x07u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x08u8; 32]));
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

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "To Delete", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(SovereignProductSlice::desktop_present_family_space(&desktop).0, 1);

    // Delete on mobile
    mobile.delete_object(id, None).unwrap();
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    // Disappears from Family Space on desktop
    assert_eq!(SovereignProductSlice::desktop_present_family_space(&desktop).0, 0);
}

#[test]
fn test_r71_27_e_work_space_remains_empty_after_family_sync() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Family Trip", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    let mut shell = NexHomeShell::new();
    shell.switch_space(SpaceType::Work);
    assert_eq!(shell.generate_home_summary(&desktop).total_objects_in_space, 0);
}

#[test]
fn test_r71_27_f_community_space_remains_empty_after_family_sync() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Family BBQ", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    let mut shell = NexHomeShell::new();
    shell.switch_space(SpaceType::Community);
    assert_eq!(shell.generate_home_summary(&desktop).total_objects_in_space, 0);
}
