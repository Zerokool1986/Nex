use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::experience::{HumanExperienceEngine, InterfaceComplexity};
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r71_30_a_photos_screen_renders_photo_cards() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Beach 2026", vec![0xAA; 4096], 10, &BTreeMap::new(), &root_actor).unwrap();

    let vm = HumanExperienceEngine::render_photos_screen(&node, SpaceType::Family, InterfaceComplexity::Standard);
    assert_eq!(vm.total_photos, 1);
    assert_eq!(vm.photo_cards[0].title, "Beach 2026");
    assert_eq!(vm.photo_cards[0].byte_size, 4096);
    assert_eq!(vm.photo_cards[0].byte_size_formatted, "4.0 KB");
}

#[test]
fn test_r71_30_b_drive_screen_renders_file_rows() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    SovereignProductSlice::mobile_create_family_document(&mut node, &proof, "Taxes_2026.pdf", vec![0xBB; 8192], 10, &BTreeMap::new(), &root_actor).unwrap();

    let vm = HumanExperienceEngine::render_drive_screen(&node, SpaceType::Family, InterfaceComplexity::Standard);
    assert_eq!(vm.total_files, 1);
    assert_eq!(vm.file_rows[0].filename, "Taxes_2026.pdf");
    assert_eq!(vm.file_rows[0].byte_size_formatted, "8.0 KB");
}

#[test]
fn test_r71_30_c_object_detail_viewmodel_rendering() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Sunset Over Mountain", b"pic".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let detail_simple = HumanExperienceEngine::render_object_detail(&node, &id, InterfaceComplexity::Simple).unwrap();
    assert_eq!(detail_simple.title, "Sunset Over Mountain");
    assert_eq!(detail_simple.space_label, "Family");
    assert!(detail_simple.status_badge.contains("Protected"));
    assert!(detail_simple.advanced_diagnostics.is_none());

    let detail_adv = HumanExperienceEngine::render_object_detail(&node, &id, InterfaceComplexity::Advanced).unwrap();
    assert!(detail_adv.advanced_diagnostics.is_some());
}

#[test]
fn test_r71_30_d_photos_and_drive_lenses_filter_by_object_type() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    // 2 Photos + 3 Docs
    for i in 1..=2 {
        SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, &format!("Pic {}", i), b"pic".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    }
    for i in 1..=3 {
        SovereignProductSlice::mobile_create_family_document(&mut node, &proof, &format!("Doc {}.txt", i), b"doc".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    let p_vm = HumanExperienceEngine::render_photos_screen(&node, SpaceType::Family, InterfaceComplexity::Simple);
    let d_vm = HumanExperienceEngine::render_drive_screen(&node, SpaceType::Family, InterfaceComplexity::Simple);

    assert_eq!(p_vm.total_photos, 2);
    assert_eq!(d_vm.total_files, 3);
}

#[test]
fn test_r71_30_e_nonexistent_object_detail_returns_error() {
    let tmp = tempdir().unwrap();
    let node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x05u8; 32]));
    let fake_id = [0xFF; 32];
    assert!(HumanExperienceEngine::render_object_detail(&node, &fake_id, InterfaceComplexity::Simple).is_err());
}

#[test]
fn test_r71_30_f_photos_lens_shows_calm_empty_state() {
    let tmp = tempdir().unwrap();
    let node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x06u8; 32]));
    let vm = HumanExperienceEngine::render_photos_screen(&node, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(vm.total_photos, 0);
    assert!(vm.photo_cards.is_empty());
}
