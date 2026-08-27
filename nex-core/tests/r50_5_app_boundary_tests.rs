use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::{NexAppApi, CoreRuntimeError};
use nex_core::object::types::ObjectType;
use nex_core::identity::types::{KeyType, OP_WRITE, OP_READ};
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r50_5_a_drive_decoupled_operations() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let ns = [0x10; 32];

    // 1. Create Folder Object
    let folder_id = node.create_object(
        ns,
        ObjectType::DriveFolder,
        BTreeMap::from([("path".into(), "/Documents".into())]),
        b"FOLDER_PAYLOAD".to_vec(),
    ).expect("Folder creation must succeed");

    // 2. Create File Inode Object
    let file_id = node.create_object(
        ns,
        ObjectType::DriveInode,
        BTreeMap::from([("path".into(), "/Documents/report.pdf".into())]),
        b"FILE_BINARY_PAYLOAD_V1".to_vec(),
    ).expect("File creation must succeed");

    // 3. Read back file object
    let file_obj = node.read_object(&file_id).expect("Read must succeed");
    assert_eq!(file_obj.payload_bytes, b"FILE_BINARY_PAYLOAD_V1");

    // 4. Mutate file
    node.mutate_object(
        file_id,
        Some(BTreeMap::from([("path".into(), "/Documents/report_final.pdf".into())])),
        Some(b"FILE_BINARY_PAYLOAD_V2".to_vec()),
        None,
    ).expect("Mutation must succeed");

    let updated_obj = node.read_object(&file_id).unwrap();
    assert_eq!(updated_obj.payload_bytes, b"FILE_BINARY_PAYLOAD_V2");
    assert_eq!(updated_obj.metadata.get("path").unwrap(), "/Documents/report_final.pdf");

    // 5. Tombstone file
    node.delete_object(file_id, None).expect("Delete must succeed");
    let tombstone_res = node.read_object(&file_id);
    assert_eq!(tombstone_res, Err(CoreRuntimeError::ObjectTombstoned));

    node.stop().unwrap();
}

#[test]
fn test_r50_5_b_chat_decoupled_operations() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let ns = [0x20; 32];

    // 1. Create Chat Channel
    let channel_id = node.create_object(
        ns,
        ObjectType::ChatChannel,
        BTreeMap::from([("channel_name".into(), "#general".into())]),
        b"CHANNEL_METADATA".to_vec(),
    ).unwrap();

    // 2. Post Messages
    let msg1_id = node.create_object(
        ns,
        ObjectType::ChatMessage,
        BTreeMap::from([("channel_id".into(), hex::encode(channel_id))]),
        b"Hello Sovereign Mesh!".to_vec(),
    ).unwrap();

    let msg2_id = node.create_object(
        ns,
        ObjectType::ChatMessage,
        BTreeMap::from([("channel_id".into(), hex::encode(channel_id))]),
        b"Nex Distributed Core Active".to_vec(),
    ).unwrap();

    let msg1 = node.read_object(&msg1_id).unwrap();
    let msg2 = node.read_object(&msg2_id).unwrap();

    assert_eq!(msg1.payload_bytes, b"Hello Sovereign Mesh!");
    assert_eq!(msg2.payload_bytes, b"Nex Distributed Core Active");

    node.stop().unwrap();
}

#[test]
fn test_r50_5_c_community_decoupled_operations() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let ns = [0x30; 32];

    // 1. Create Community Object
    let post_id = node.create_object(
        ns,
        ObjectType::Community,
        BTreeMap::from([("title".into(), "Sovereignty RFC".into())]),
        b"Post Body Content".to_vec(),
    ).unwrap();

    // 2. Create Member Role
    let reply_id = node.create_object(
        ns,
        ObjectType::MemberRole,
        BTreeMap::from([("post_id".into(), hex::encode(post_id))]),
        b"Strongly Agree with RFC".to_vec(),
    ).unwrap();

    let post = node.read_object(&post_id).unwrap();
    let reply = node.read_object(&reply_id).unwrap();

    assert_eq!(post.payload_bytes, b"Post Body Content");
    assert_eq!(reply.payload_bytes, b"Strongly Agree with RFC");

    node.stop().unwrap();
}

#[test]
fn test_r50_5_d_photos_decoupled_operations() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let ns = [0x40; 32];

    // 1. Create Album
    let album_id = node.create_object(
        ns,
        ObjectType::PhotoAlbum,
        BTreeMap::from([("title".into(), "Summer 2026".into())]),
        b"ALBUM_ROOT".to_vec(),
    ).unwrap();

    // 2. Import Photo with EXIF metadata
    let photo_id = node.create_object(
        ns,
        ObjectType::PhotoMedia,
        BTreeMap::from([
            ("album_id".into(), hex::encode(album_id)),
            ("camera_make".into(), "Sony".into()),
            ("camera_model".into(), "A7IV".into()),
            ("iso".into(), "100".into()),
            ("width".into(), "7008".into()),
            ("height".into(), "4672".into()),
        ]),
        b"PHOTO_RAW_PIXELS_OR_CAS_ROOT".to_vec(),
    ).unwrap();

    let photo = node.read_object(&photo_id).unwrap();
    assert_eq!(photo.metadata.get("camera_make").unwrap(), "Sony");
    assert_eq!(photo.metadata.get("iso").unwrap(), "100");
    assert_eq!(photo.payload_bytes, b"PHOTO_RAW_PIXELS_OR_CAS_ROOT");

    node.stop().unwrap();
}

#[test]
fn test_r50_5_e_sandboxed_extension_capability_enforcement() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key_owner = SigningKey::generate(&mut csprng);
    let key_extension = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key_owner);
    node.start().unwrap();

    let ns_a = [0xAA; 32];
    let ns_b = [0xBB; 32];

    // Create object in Namespace B
    let obj_b_id = node.create_object(
        ns_b,
        ObjectType::DriveInode,
        BTreeMap::new(),
        b"CONFIDENTIAL_B".to_vec(),
    ).unwrap();

    // Extension actor
    let ext_actor = derive_actor_id(KeyType::Ed25519, key_extension.verifying_key().as_bytes());

    // Delegate capability ONLY for Namespace A
    let proof_for_a = node.delegate_capability(
        ext_actor,
        ns_a,
        None,
        OP_WRITE | OP_READ,
        (0, 100),
    ).unwrap();

    // Attack: Extension tries to mutate Object B in Namespace B using Proof for Namespace A
    let attack_res = node.mutate_object(
        obj_b_id,
        None,
        Some(b"TAMPERED_B".to_vec()),
        Some(proof_for_a),
    );

    assert!(attack_res.is_err(), "Extension must be denied access to Namespace B");
    match attack_res {
        Err(CoreRuntimeError::Unauthorized(_)) => {}
        other => panic!("Expected Unauthorized error, got: {:?}", other),
    }

    node.stop().unwrap();
}

#[test]
fn test_r50_5_f_cross_app_multi_tenant_isolation() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let node_arc = Arc::new(Mutex::new(node));
    let mut handles = Vec::new();

    // 4 Concurrent Tenants across 4 Namespaces
    for app_idx in 0..4u8 {
        let n = Arc::clone(&node_arc);
        let handle = thread::spawn(move || {
            let ns = [app_idx; 32];
            for i in 0..25 {
                let mut guard = n.lock().unwrap();
                let obj_type = match app_idx {
                    0 => ObjectType::DriveFolder,
                    1 => ObjectType::ChatChannel,
                    2 => ObjectType::Community,
                    _ => ObjectType::PhotoMedia,
                };
                guard.create_object(
                    ns,
                    obj_type,
                    BTreeMap::from([("index".into(), i.to_string())]),
                    format!("APP_{}_PAYLOAD_{}", app_idx, i).into_bytes(),
                ).unwrap();
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let mut guard = node_arc.lock().unwrap();
    assert_eq!(guard.state.object_store.len(), 100, "100 objects must be created across 4 namespaces");

    // Verify namespace isolation: exactly 25 objects per namespace
    for app_idx in 0..4u8 {
        let ns = [app_idx; 32];
        let count = guard.state.object_store.values().filter(|o| o.namespace == ns).count();
        assert_eq!(count, 25, "Namespace {} must have exactly 25 objects", app_idx);
    }

    guard.stop().unwrap();
}
