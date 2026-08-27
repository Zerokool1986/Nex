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
fn test_r71_31_a_experience_slider_stages_disclosure_on_home_feed() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Beach", b"pic".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let v_simple = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Simple);
    let v_std = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Standard);
    let v_adv = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Advanced);
    let v_exp = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Expert);

    assert_eq!(v_simple.feed_items[0].status_badge, "Protected");
    assert!(v_std.feed_items[0].status_badge.contains("Synced"));
    assert!(v_adv.feed_items[0].status_badge.contains("CAS:"));
    assert!(v_exp.feed_items[0].status_badge.contains("SMT Node"));
}

#[test]
fn test_r71_31_b_person_panel_accessible_without_exposing_cryptographic_blobs() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x02u8; 32]));
    node.start().unwrap();

    let friend = [0x55u8; 32];
    let panel = nex_core::runtime::panels::ContextualPanelsEngine::project_person_panel(&node, &friend, "Bob");
    assert_eq!(panel.display_name, "Bob");
    assert!(panel.direct_chat_available);
    assert_eq!(panel.shared_objects_count, 0);
}

#[test]
fn test_r71_31_c_device_panel_projects_clean_health_summary() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x03u8; 32]);
    let pk = key.verifying_key().to_bytes();
    let actor_id = derive_actor_id(KeyType::Ed25519, &pk);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let dev_panel = nex_core::runtime::panels::ContextualPanelsEngine::project_device_panel(&node, &actor_id, None, false);
    assert!(dev_panel.is_local_device);
    assert!(!dev_panel.is_revoked);
}

#[test]
fn test_r71_31_d_storage_panel_breakdown_by_category() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x04u8; 32]));
    node.start().unwrap();

    let root_actor = node.identity.actor_id;
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
        issuer_pubkey: Some(SigningKey::from_bytes(&[0x04u8; 32]).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&[0x04u8; 32]).sign(&token_hash).to_bytes().to_vec(),
    };

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Photo", vec![0; 500], 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::mobile_create_family_document(&mut node, &proof, "Doc", vec![0; 300], 10, &BTreeMap::new(), &root_actor).unwrap();

    let storage = nex_core::runtime::panels::ContextualPanelsEngine::project_storage_panel(&node);
    assert_eq!(storage.total_used_bytes, 800);
    assert_eq!(storage.photos_bytes, 500);
    assert_eq!(storage.drive_bytes, 300);
}

#[test]
fn test_r71_31_e_slider_level_does_not_mutate_underlying_state() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x05u8; 32]));
    node.start().unwrap();

    let root_actor = node.identity.actor_id;
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
        issuer_pubkey: Some(SigningKey::from_bytes(&[0x05u8; 32]).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&[0x05u8; 32]).sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Immutable", b"abc".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let _ = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Simple);
    let _ = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Expert);

    assert_eq!(node.state.object_store.len(), 1);
    assert_eq!(node.state.object_store.get(&id).unwrap().payload_bytes, b"abc");
}

#[test]
fn test_r71_31_f_expert_mode_exposes_owner_actor_hashes() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x06u8; 32]));
    node.start().unwrap();

    let root_actor = node.identity.actor_id;
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
        issuer_pubkey: Some(SigningKey::from_bytes(&[0x06u8; 32]).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&[0x06u8; 32]).sign(&token_hash).to_bytes().to_vec(),
    };

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Test", b"pic".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let vm_exp = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Expert);
    assert!(vm_exp.feed_items[0].status_badge.contains("Owner:"));
}
