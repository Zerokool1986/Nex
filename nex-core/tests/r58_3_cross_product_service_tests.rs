use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::platform::*;
use nex_core::object::types::ObjectType;
use nex_core::api::NexAppApi;
use std::collections::BTreeMap;

#[test]
fn test_r58_3_a_nex_web_gateway_uri_resolution() {
    let dir = tempdir().unwrap();
    let seed = [71u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns_web = [0xBB; 32];
    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), "/index.html".to_string());
    meta.insert("content-type".to_string(), "text/html".to_string());
    node.create_object(ns_web, ObjectType::Synthetic(20), meta, b"<h1>Hello Nex Web</h1>".to_vec()).unwrap();

    let uri_str = format!("nex://{}/{}/index.html", hex::encode(node.identity.actor_id), hex::encode(ns_web));
    let uri = NexUri::parse(&uri_str).unwrap();

    let resolved = NexUriResolver::resolve_uri(&node, &uri).unwrap();
    assert_eq!(resolved.payload_bytes, b"<h1>Hello Nex Web</h1>");
    assert_eq!(resolved.metadata.get("content-type").unwrap(), "text/html");
}

#[test]
fn test_r58_3_b_nex_maps_spatial_waypoint_workflow() {
    let dir = tempdir().unwrap();
    let seed = [72u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns_maps = [0xAA; 32];
    let mut meta = BTreeMap::new();
    meta.insert("name".to_string(), "Summit Waypoint".to_string());
    meta.insert("lat".to_string(), "45.0".to_string());
    meta.insert("lon".to_string(), "10.0".to_string());

    node.create_object(ns_maps, ObjectType::Synthetic(10), meta, b"Waypoint details".to_vec()).unwrap();

    let results = SpatialMapEngine::query_bounding_box(&node, 44.0, 46.0, 9.0, 11.0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.get("name").unwrap(), "Summit Waypoint");
}

#[test]
fn test_r58_3_c_group_federation_shared_access() {
    let group_root_seed = [73u8; 32];
    let group_root = SigningKey::from_bytes(&group_root_seed);

    let member_seed = [74u8; 32];
    let member_key = SigningKey::from_bytes(&member_seed);
    let member_actor = nex_core::identity::verifier::derive_actor_id(
        nex_core::identity::types::KeyType::Ed25519,
        &member_key.verifying_key().to_bytes(),
    );

    let group_id = [0xCC; 32];
    let proof = GroupFederationEngine::create_group_capability_token(
        &group_root,
        member_actor,
        group_id,
        nex_core::identity::types::OP_ALL,
    );

    assert_eq!(proof.token.namespace, group_id);
    assert_eq!(proof.token.allowed_operations, nex_core::identity::types::OP_ALL);
}

#[test]
fn test_r58_3_d_petname_directed_routing() {
    let mut dir = PetnameDirectory::new();
    let peer_actor = [0xEE; 32];
    dir.set_petname("AlicePhone", peer_actor);

    let resolved_actor = dir.resolve_petname("alicephone").unwrap();
    assert_eq!(resolved_actor, peer_actor);
}

#[test]
fn test_r58_3_e_offline_outbox_reconnection_sync() {
    let mut outbox = OfflineOutbox::new();
    let ns = [0xDD; 32];
    outbox.enqueue(ns, ObjectType::ChatChannel, BTreeMap::new(), b"Message while in tunnel".to_vec());

    let dir = tempdir().unwrap();
    let seed = [75u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    assert_eq!(outbox.flush_to_node(&mut node).unwrap(), 1);
    assert_eq!(node.state.object_store.len(), 1);
}

#[test]
fn test_r58_3_f_master_cross_product_consistency() {
    let dir = tempdir().unwrap();
    let seed = [76u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    // Ingest drive, photos, chat, maps, and web objects
    node.create_object([0x10; 32], ObjectType::DriveInode, BTreeMap::new(), b"Drive".to_vec()).unwrap();
    node.create_object([0x20; 32], ObjectType::PhotoAlbum, BTreeMap::new(), b"Photo".to_vec()).unwrap();
    node.create_object([0x30; 32], ObjectType::ChatChannel, BTreeMap::new(), b"Chat".to_vec()).unwrap();
    node.create_object([0xAA; 32], ObjectType::Synthetic(10), BTreeMap::new(), b"Map".to_vec()).unwrap();
    node.create_object([0xBB; 32], ObjectType::Synthetic(20), BTreeMap::new(), b"Web".to_vec()).unwrap();

    assert_eq!(node.state.object_store.len(), 5);
}
