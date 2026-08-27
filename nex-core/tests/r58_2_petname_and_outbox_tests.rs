use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::platform::*;
use nex_core::object::types::ObjectType;
use std::collections::BTreeMap;

#[test]
fn test_r58_2_a_petname_directory_alias_resolution() {
    let mut dir = PetnameDirectory::new();
    let alice_actor = [0x11u8; 32];
    let bob_actor = [0x22u8; 32];

    dir.set_petname("Alice", alice_actor);
    dir.set_petname("Bob", bob_actor);

    // Case-insensitive resolution
    assert_eq!(dir.resolve_petname("alice"), Some(alice_actor));
    assert_eq!(dir.resolve_petname("ALICE"), Some(alice_actor));
    assert_eq!(dir.resolve_petname("bob"), Some(bob_actor));
    assert_eq!(dir.resolve_petname("charlie"), None);
}

#[test]
fn test_r58_2_b_petname_directory_alias_update() {
    let mut dir = PetnameDirectory::new();
    let old_key = [0x11u8; 32];
    let new_key = [0x33u8; 32];

    dir.set_petname("WorkDevice", old_key);
    assert_eq!(dir.resolve_petname("workdevice"), Some(old_key));

    dir.set_petname("WorkDevice", new_key);
    assert_eq!(dir.resolve_petname("workdevice"), Some(new_key));
}

#[test]
fn test_r58_2_c_offline_outbox_queueing() {
    let mut outbox = OfflineOutbox::new();
    let ns = [0x77u8; 32];

    let mut meta1 = BTreeMap::new();
    meta1.insert("title".to_string(), "Doc 1".to_string());
    let id1 = outbox.enqueue(ns, ObjectType::DriveInode, meta1, b"Payload 1".to_vec());

    let mut meta2 = BTreeMap::new();
    meta2.insert("title".to_string(), "Doc 2".to_string());
    let id2 = outbox.enqueue(ns, ObjectType::DriveInode, meta2, b"Payload 2".to_vec());

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(outbox.queue.len(), 2);
}

#[test]
fn test_r58_2_d_offline_outbox_flush_to_node() {
    let mut outbox = OfflineOutbox::new();
    let ns = [0x78u8; 32];

    let mut meta1 = BTreeMap::new();
    meta1.insert("msg".to_string(), "Offline chat 1".to_string());
    outbox.enqueue(ns, ObjectType::ChatChannel, meta1, b"Hello while offline".to_vec());

    let mut meta2 = BTreeMap::new();
    meta2.insert("msg".to_string(), "Offline chat 2".to_string());
    outbox.enqueue(ns, ObjectType::ChatChannel, meta2, b"Another offline message".to_vec());

    let dir = tempdir().unwrap();
    let seed = [61u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let flushed = outbox.flush_to_node(&mut node).unwrap();
    assert_eq!(flushed, 2);
    assert_eq!(outbox.queue.len(), 0, "Outbox queue must be empty after flush");
    assert_eq!(node.state.object_store.len(), 2, "Node must have 2 objects");
}

#[test]
fn test_r58_2_e_offline_outbox_multi_namespace_drain() {
    let mut outbox = OfflineOutbox::new();
    let ns_drive = [0x10u8; 32];
    let ns_chat = [0x20u8; 32];
    let ns_photos = [0x30u8; 32];

    outbox.enqueue(ns_drive, ObjectType::DriveInode, BTreeMap::new(), b"Drive file".to_vec());
    outbox.enqueue(ns_chat, ObjectType::ChatChannel, BTreeMap::new(), b"Chat text".to_vec());
    outbox.enqueue(ns_photos, ObjectType::PhotoAlbum, BTreeMap::new(), b"Photo item".to_vec());

    let dir = tempdir().unwrap();
    let seed = [62u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let count = outbox.flush_to_node(&mut node).unwrap();
    assert_eq!(count, 3);
    assert_eq!(node.state.object_store.len(), 3);
}

#[test]
fn test_r58_2_f_zero_regression_across_outbox_lifecycle() {
    let mut outbox = OfflineOutbox::new();
    let dir = tempdir().unwrap();
    let seed = [63u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    // Flush empty outbox
    let flushed = outbox.flush_to_node(&mut node).unwrap();
    assert_eq!(flushed, 0);
}
