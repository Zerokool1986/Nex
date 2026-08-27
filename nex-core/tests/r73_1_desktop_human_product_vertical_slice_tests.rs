use std::fs;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::product::desktop_app::{DesktopAppSession, DesktopNavigationTab};
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::ObjectType;

#[test]
fn test_r73_1_a_desktop_session_initializes_with_home_and_family_space() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x01u8; 32]));
    node.start().unwrap();

    let session = DesktopAppSession::new();
    assert_eq!(session.active_tab, DesktopNavigationTab::Home);
    assert_eq!(session.active_space, SpaceType::Family);

    let view = session.render_view_string(&node);
    assert!(view.contains("NEX DESKTOP"));
    assert!(view.contains("HOME"));
    assert!(view.contains("Family"));
}

#[test]
fn test_r73_1_b_real_filesystem_image_import_to_family_space() {
    let tmp_node = tempdir().unwrap();
    let tmp_files = tempdir().unwrap();

    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp_node.path(), SigningKey::from_bytes(&root_seed));
    node.start().unwrap();

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

    // Create a real physical image file on disk
    let file_path = tmp_files.path().join("vacation_mountain.jpg");
    let test_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x11, 0x22, 0x33, 0x44];
    fs::write(&file_path, &test_bytes).unwrap();

    let mut session = DesktopAppSession::new();
    let obj_id = session.import_local_file(
        &mut node,
        &file_path,
        &proof,
        &root_actor,
        10,
    ).expect("Physical file import failed");

    // Verify object committed to canonical state
    assert!(node.state.object_store.contains_key(&obj_id));
    let stored = node.state.object_store.get(&obj_id).unwrap();
    assert_eq!(stored.object_type, ObjectType::PhotoMedia);
    assert_eq!(stored.payload_bytes, test_bytes);
}

#[test]
fn test_r73_1_c_imported_photo_renders_immediately_in_photos_lens() {
    let tmp_node = tempdir().unwrap();
    let tmp_files = tempdir().unwrap();

    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp_node.path(), SigningKey::from_bytes(&root_seed));
    node.start().unwrap();

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

    let file_path = tmp_files.path().join("beach_cabin.png");
    fs::write(&file_path, b"png_magic_bytes_data").unwrap();

    let mut session = DesktopAppSession::new();
    session.import_local_file(&mut node, &file_path, &proof, &root_actor, 10).unwrap();

    // Switch to Photos lens
    session.select_tab(DesktopNavigationTab::Photos);
    let photos_view = session.render_view_string(&node);

    assert!(photos_view.contains("PHOTOS LENS"));
    assert!(photos_view.contains("beach_cabin.png"));
    assert!(photos_view.contains("Total Photos : 1"));
}

#[test]
fn test_r73_1_d_universal_inspector_renders_canonical_metadata() {
    let tmp_node = tempdir().unwrap();
    let tmp_files = tempdir().unwrap();

    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp_node.path(), SigningKey::from_bytes(&root_seed));
    node.start().unwrap();

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

    let file_path = tmp_files.path().join("sunset.jpg");
    fs::write(&file_path, vec![0x99; 1024]).unwrap();

    let mut session = DesktopAppSession::new();
    let obj_id = session.import_local_file(&mut node, &file_path, &proof, &root_actor, 10).unwrap();

    session.inspect_object(obj_id);
    let insp_view = session.render_view_string(&node);

    assert!(insp_view.contains("UNIVERSAL OBJECT INSPECTOR"));
    assert!(insp_view.contains("sunset.jpg"));
    assert!(insp_view.contains("1.0 KB"));
    assert!(insp_view.contains("Replicas   : 2"));
}

#[test]
fn test_r73_1_e_person_and_device_surfaces_render_truthful_state() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x05u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let mut session = DesktopAppSession::new();

    // 1. Person surface
    session.open_person([0xAA; 32], "Amy");
    let person_view = session.render_view_string(&node);
    assert!(person_view.contains("PERSON PANEL — Amy"));
    assert!(person_view.contains("Verified Sovereign Peer"));

    // 2. Device surface
    session.open_device(root_actor, "Chris's Primary Desktop");
    let dev_view = session.render_view_string(&node);
    assert!(dev_view.contains("DEVICE PANEL — Chris's Primary Desktop"));
    assert!(dev_view.contains("This Device"));
}

#[test]
fn test_r73_1_f_experience_slider_changes_presentation_not_authority() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x06u8; 32]);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let mut session = DesktopAppSession::new();
    session.select_tab(DesktopNavigationTab::Settings);

    // Simple mode
    session.set_complexity_slider(InterfaceComplexity::Simple);
    let view_simple = session.render_view_string(&node);
    assert!(!view_simple.contains("Cryptographic & Storage Diagnostics"));

    // Expert mode
    session.set_complexity_slider(InterfaceComplexity::Expert);
    let view_expert = session.render_view_string(&node);
    assert!(view_expert.contains("Cryptographic & Storage Diagnostics"));
    assert!(view_expert.contains("SMT State Root Inspector"));
}
