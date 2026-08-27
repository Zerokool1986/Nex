use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::runtime::panels::ContextualPanelsEngine;
use nex_core::object::types::ObjectType;
use nex_core::identity::types::{DeviceCertificate, KeyType};
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r71_22_a_person_panel_projection() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x01; 32]));
    node.start().unwrap();

    let friend_actor = [0xAA; 32];

    let person = ContextualPanelsEngine::project_person_panel(&node, &friend_actor, "Amy");
    assert_eq!(person.display_name, "Amy");
    assert_eq!(person.actor_id, friend_actor);
    assert_eq!(person.shared_objects_count, 0);
    assert!(person.direct_chat_available);
}

#[test]
fn test_r71_22_b_device_panel_local_device_detection() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x02; 32]);
    let pk = key.verifying_key().to_bytes();
    let actor_id = derive_actor_id(KeyType::Ed25519, &pk);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let dev_panel = ContextualPanelsEngine::project_device_panel(&node, &actor_id, None, false);
    assert!(dev_panel.is_local_device);
    assert!(!dev_panel.is_revoked);
}

#[test]
fn test_r71_22_c_device_panel_certificate_and_revocation() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x03; 32]));
    node.start().unwrap();

    let remote_actor = [0x55; 32];
    let cert = DeviceCertificate {
        master_actor_id: [0x11; 32],
        device_actor_id: remote_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: None,
        signature: vec![0; 64],
    };

    let dev_panel = ContextualPanelsEngine::project_device_panel(&node, &remote_actor, Some(&cert), true);
    assert!(!dev_panel.is_local_device);
    assert_eq!(dev_panel.not_before_epoch, 100);
    assert_eq!(dev_panel.expires_at_epoch, 500);
    assert!(dev_panel.is_revoked);
}

#[test]
fn test_r71_22_d_storage_panel_category_breakdown() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x04; 32]));
    node.start().unwrap();

    node.create_object([0x01; 32], ObjectType::PhotoMedia, BTreeMap::new(), vec![0xAA; 1000]).unwrap();
    node.create_object([0x01; 32], ObjectType::DriveInode, BTreeMap::new(), vec![0xBB; 500]).unwrap();
    node.create_object([0x01; 32], ObjectType::VaultItem, BTreeMap::new(), vec![0xCC; 200]).unwrap();

    let storage = ContextualPanelsEngine::project_storage_panel(&node);
    assert_eq!(storage.total_used_bytes, 1700);
    assert_eq!(storage.photos_bytes, 1000);
    assert_eq!(storage.drive_bytes, 500);
    assert_eq!(storage.vault_bytes, 200);
    assert_eq!(storage.objects_count, 3);
}

#[test]
fn test_r71_22_e_storage_panel_tombstoned_filtering() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x05; 32]));
    node.start().unwrap();

    let id = node.create_object([0x01; 32], ObjectType::PhotoMedia, BTreeMap::new(), vec![0; 800]).unwrap();
    assert_eq!(ContextualPanelsEngine::project_storage_panel(&node).total_used_bytes, 800);

    node.delete_object(id, None).unwrap();
    assert_eq!(ContextualPanelsEngine::project_storage_panel(&node).total_used_bytes, 0);
}

#[test]
fn test_r71_22_f_empty_storage_panel() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x06; 32]));
    node.start().unwrap();

    let storage = ContextualPanelsEngine::project_storage_panel(&node);
    assert_eq!(storage.total_used_bytes, 0);
    assert_eq!(storage.objects_count, 0);
}
