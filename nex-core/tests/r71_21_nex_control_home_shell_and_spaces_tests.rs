use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::object::types::ObjectType;

#[test]
fn test_r71_21_a_default_space_is_personal() {
    let shell = NexHomeShell::new();
    assert_eq!(shell.active_space, SpaceType::Personal);
}

#[test]
fn test_r71_21_b_switch_spaces_updates_active_view() {
    let mut shell = NexHomeShell::new();
    shell.switch_space(SpaceType::Family);
    assert_eq!(shell.active_space, SpaceType::Family);
    shell.switch_space(SpaceType::Work);
    assert_eq!(shell.active_space, SpaceType::Work);
}

#[test]
fn test_r71_21_c_home_summary_aggregates_space_objects() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x01; 32]));
    node.start().unwrap();

    let mut shell = NexHomeShell::new();

    // Create 2 Personal objects
    let personal_ns = NexHomeShell::space_to_namespace(SpaceType::Personal);
    node.create_object(personal_ns, ObjectType::PhotoMedia, BTreeMap::new(), b"Photo 1".to_vec()).unwrap();
    node.create_object(personal_ns, ObjectType::DriveInode, BTreeMap::new(), b"Doc 1".to_vec()).unwrap();

    // Create 1 Family object
    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    node.create_object(family_ns, ObjectType::PhotoMedia, BTreeMap::new(), b"Family Pic".to_vec()).unwrap();

    // In Personal space -> count 2
    let sum_p = shell.generate_home_summary(&node);
    assert_eq!(sum_p.total_objects_in_space, 2);

    // Switch to Family space -> count 1
    shell.switch_space(SpaceType::Family);
    let sum_f = shell.generate_home_summary(&node);
    assert_eq!(sum_f.total_objects_in_space, 1);
}

#[test]
fn test_r71_21_d_recent_activity_chronological_ordering() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x02; 32]));
    node.start().unwrap();

    let shell = NexHomeShell::new();
    let ns = NexHomeShell::space_to_namespace(SpaceType::Personal);

    node.create_object(ns, ObjectType::DriveInode, BTreeMap::new(), b"First".to_vec()).unwrap();
    node.create_object(ns, ObjectType::DriveInode, BTreeMap::new(), b"Second".to_vec()).unwrap();

    let items = shell.recent_activity_for_space(&node, SpaceType::Personal);
    assert_eq!(items.len(), 2);
    assert!(items[0].timestamp_epoch >= items[1].timestamp_epoch);
}

#[test]
fn test_r71_21_e_empty_space_activity_is_empty() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x03; 32]));
    node.start().unwrap();

    let shell = NexHomeShell::new();
    let items = shell.recent_activity_for_space(&node, SpaceType::Community);
    assert!(items.is_empty());
}

#[test]
fn test_r71_21_f_tombstoned_objects_filtered_from_feed() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x04; 32]));
    node.start().unwrap();

    let shell = NexHomeShell::new();
    let ns = NexHomeShell::space_to_namespace(SpaceType::Personal);

    let obj_id = node.create_object(ns, ObjectType::DriveInode, BTreeMap::new(), b"To Delete".to_vec()).unwrap();
    assert_eq!(shell.recent_activity_for_space(&node, SpaceType::Personal).len(), 1);

    node.delete_object(obj_id, None).unwrap();
    assert_eq!(shell.recent_activity_for_space(&node, SpaceType::Personal).len(), 0);
}
