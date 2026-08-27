use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::experience::{HumanExperienceEngine, InterfaceComplexity};
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::api::NexAppApi;

#[test]
fn test_r71_29_a_home_screen_renders_active_space_viewmodel() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Family Trip", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let vm_simple = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(vm_simple.active_space, SpaceType::Family);
    assert_eq!(vm_simple.total_items_in_space, 1);
    assert_eq!(vm_simple.feed_items.len(), 1);
    assert_eq!(vm_simple.feed_items[0].title, "Family Trip");
    assert_eq!(vm_simple.feed_items[0].status_badge, "Protected");
}

#[test]
fn test_r71_29_b_dynamic_space_switching_isolates_feed() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
    node.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let personal_ns = NexHomeShell::space_to_namespace(SpaceType::Personal);

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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&hash_capability_token(&CapabilityToken {
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&hash_capability_token(&CapabilityToken {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &token_fam, "Family Item", b"fam".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    nex_core::runtime::dispatcher::UiActionDispatcher::dispatch_ui_create_object(
        &mut node,
        &token_per,
        personal_ns,
        nex_core::object::types::ObjectType::DriveInode,
        BTreeMap::new(),
        b"per".to_vec(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).unwrap();

    let vm_fam = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Standard);
    let vm_per = HumanExperienceEngine::render_home_screen(&node, SpaceType::Personal, InterfaceComplexity::Standard);
    let vm_work = HumanExperienceEngine::render_home_screen(&node, SpaceType::Work, InterfaceComplexity::Standard);

    assert_eq!(vm_fam.feed_items.len(), 1);
    assert_eq!(vm_fam.feed_items[0].title, "Family Item");
    assert_eq!(vm_per.feed_items.len(), 1);
    assert_eq!(vm_work.feed_items.len(), 0);
}

#[test]
fn test_r71_29_c_tombstoned_objects_hidden_from_human_view() {
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

    let (obj_id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "To Delete", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    assert_eq!(HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Simple).feed_items.len(), 1);

    node.delete_object(obj_id, None).unwrap();
    assert_eq!(HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Simple).feed_items.len(), 0);
}

#[test]
fn test_r71_29_d_available_spaces_list_is_complete() {
    let tmp = tempdir().unwrap();
    let node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x04u8; 32]));
    let vm = HumanExperienceEngine::render_home_screen(&node, SpaceType::Personal, InterfaceComplexity::Simple);
    assert_eq!(vm.available_spaces.len(), 5);
}

#[test]
fn test_r71_29_e_empty_space_presents_zero_items_calm_state() {
    let tmp = tempdir().unwrap();
    let node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x05u8; 32]));
    let vm = HumanExperienceEngine::render_home_screen(&node, SpaceType::Community, InterfaceComplexity::Simple);
    assert_eq!(vm.total_items_in_space, 0);
    assert!(vm.feed_items.is_empty());
}

#[test]
fn test_r71_29_f_feed_item_displays_formatted_badges() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x06u8; 32];
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

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Sunset", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let vm = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Standard);
    assert!(vm.feed_items[0].shared_badge.contains("Family"));
    assert!(vm.feed_items[0].status_badge.contains("Synced"));
}
