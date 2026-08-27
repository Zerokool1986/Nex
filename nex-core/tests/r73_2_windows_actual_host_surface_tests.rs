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
fn test_r73_2_a_application_session_initializes_with_desktop_host_context() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x01u8; 32]));
    node.start().unwrap();

    let session = DesktopAppSession::new();
    assert_eq!(session.active_tab, DesktopNavigationTab::Home);
    assert_eq!(session.active_space, SpaceType::Family);
    assert_eq!(session.complexity, InterfaceComplexity::Standard);
    assert!(!session.is_hardware_keystore_verified); // Truthful desktop software key default

    let view = session.render_view_string(&node);
    assert!(view.contains("NEX DESKTOP"));
    assert!(view.contains("Family"));
}

#[test]
fn test_r73_2_b_real_filesystem_file_selection_and_canonical_ingest() {
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

    // Create a real physical file selected via file dialog path
    let file_path = tmp_files.path().join("family_cabin_2026.jpg");
    let test_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x00, 0x01];
    fs::write(&file_path, &test_bytes).unwrap();

    let mut session = DesktopAppSession::new();
    let obj_id = session.import_local_file(
        &mut node,
        &file_path,
        &proof,
        &root_actor,
        10,
    ).expect("Real filesystem ingestion failed");

    // Assert canonical object state
    let obj = node.state.object_store.get(&obj_id).expect("Object must exist in canonical state");
    assert_eq!(obj.object_type, ObjectType::PhotoMedia);
    assert_eq!(obj.payload_bytes, test_bytes);
    assert_eq!(obj.metadata.get("title").unwrap(), "family_cabin_2026.jpg");
}

#[test]
fn test_r73_2_c_photos_surface_renders_real_imported_image_metadata() {
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

    let file_path = tmp_files.path().join("lake_view.jpg");
    fs::write(&file_path, vec![0xCA; 2048]).unwrap();

    let mut session = DesktopAppSession::new();
    session.import_local_file(&mut node, &file_path, &proof, &root_actor, 10).unwrap();

    session.select_tab(DesktopNavigationTab::Photos);
    let view = session.render_view_string(&node);

    assert!(view.contains("PHOTOS LENS"));
    assert!(view.contains("lake_view.jpg"));
    assert!(view.contains("2.0 KB"));
    assert!(view.contains("Total Photos : 1"));
}

#[test]
fn test_r73_2_d_universal_inspector_exposes_canonical_dag_and_cas_invariants() {
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

    let file_path = tmp_files.path().join("deed.pdf");
    fs::write(&file_path, vec![0xEE; 4096]).unwrap();

    let mut session = DesktopAppSession::new();
    let obj_id = session.import_local_file(&mut node, &file_path, &proof, &root_actor, 10).unwrap();

    session.inspect_object(obj_id);
    session.set_complexity_slider(InterfaceComplexity::Advanced);

    let view = session.render_view_string(&node);
    assert!(view.contains("UNIVERSAL OBJECT INSPECTOR"));
    assert!(view.contains("deed.pdf"));
    assert!(view.contains("DriveInode"));
    assert!(view.contains("CAS Chunks: 1"));
    assert!(view.contains("Replicas   : 2"));
}

#[test]
fn test_r73_2_e_person_and_device_surfaces_enforce_truthful_evidence_claims() {
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

    // 2. Device surface: Truthful reporting that Hardware TEE is not yet verified on this host
    session.open_device(root_actor, "Chris's Desktop Station");
    let dev_view = session.render_view_string(&node);
    assert!(dev_view.contains("DEVICE PANEL — Chris's Desktop Station"));
    assert!(dev_view.contains("Hardware TEE: Not Verified on this Host"));
}

#[test]
fn test_r73_2_f_experience_slider_preserves_strict_security_and_state_invariants() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x06u8; 32]);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let mut session = DesktopAppSession::new();
    session.select_tab(DesktopNavigationTab::Settings);

    // Simple mode: Calm, reassuring presentation
    session.set_complexity_slider(InterfaceComplexity::Simple);
    let view_simple = session.render_view_string(&node);
    assert!(!view_simple.contains("Cryptographic & Storage Diagnostics"));

    // Expert mode: Raw cryptographic roots exposed
    session.set_complexity_slider(InterfaceComplexity::Expert);
    let view_expert = session.render_view_string(&node);
    assert!(view_expert.contains("Cryptographic & Storage Diagnostics"));
    assert!(view_expert.contains("SMT State Root Inspector"));
    assert!(view_expert.contains("Write-Ahead Log"));
}
