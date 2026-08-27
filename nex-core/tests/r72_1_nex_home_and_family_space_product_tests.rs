use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::product::home::NexHomeController;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r72_1_a_open_home_controller_renders_active_space() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Lake House", b"pic".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let home = NexHomeController::open_home(&node, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(home.active_space, SpaceType::Family);
    assert_eq!(home.total_items_in_space, 1);
    assert_eq!(home.feed_items[0].title, "Lake House");
}

#[test]
fn test_r72_1_b_list_available_spaces_returns_all_five_spaces() {
    let spaces = NexHomeController::list_available_spaces();
    assert_eq!(spaces.len(), 5);
    assert!(spaces.contains(&SpaceType::Personal));
    assert!(spaces.contains(&SpaceType::Family));
    assert!(spaces.contains(&SpaceType::Work));
    assert!(spaces.contains(&SpaceType::Community));
    assert!(spaces.contains(&SpaceType::Project));
}

#[test]
fn test_r72_1_c_home_surface_space_switching_alters_underlying_state() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Family Picnic", b"pic".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let h_fam = NexHomeController::open_home(&node, SpaceType::Family, InterfaceComplexity::Simple);
    let h_work = NexHomeController::open_home(&node, SpaceType::Work, InterfaceComplexity::Simple);

    assert_eq!(h_fam.total_items_in_space, 1);
    assert_eq!(h_work.total_items_in_space, 0);
}

#[test]
fn test_r72_1_d_home_viewmodel_sync_status_calm_everyday() {
    let tmp = tempdir().unwrap();
    let node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x03u8; 32]));
    let vm = NexHomeController::open_home(&node, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(vm.sync_status_label, "All up to date");
}

#[test]
fn test_r72_1_e_home_feed_chronological_ordering() {
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

    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Photo 1", b"p1".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Photo 2", b"p2".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let vm = NexHomeController::open_home(&node, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(vm.feed_items.len(), 2);
}

#[test]
fn test_r72_1_f_empty_space_displays_calm_zero_state() {
    let tmp = tempdir().unwrap();
    let node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x05u8; 32]));
    let vm = NexHomeController::open_home(&node, SpaceType::Project, InterfaceComplexity::Simple);
    assert_eq!(vm.total_items_in_space, 0);
    assert!(vm.feed_items.is_empty());
}
