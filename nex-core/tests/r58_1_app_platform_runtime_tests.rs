use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::platform::*;
use nex_core::object::types::ObjectType;
use nex_core::api::NexAppApi;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::types::{KeyType, OP_READ, OP_WRITE};
use std::collections::BTreeMap;

#[test]
fn test_r58_1_a_nex_uri_parsing() {
    let actor_hex = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
    let ns_hex = "a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0";
    let raw = format!("nex://{}/{}/documents/report.pdf", actor_hex, ns_hex);

    let uri = NexUri::parse(&raw).unwrap();
    assert_eq!(hex::encode(uri.actor_id), actor_hex);
    assert_eq!(hex::encode(uri.namespace), ns_hex);
    assert_eq!(uri.path, "/documents/report.pdf");

    // Invalid scheme
    assert!(NexUri::parse("http://example.com").is_err());
    // Missing namespace
    assert!(NexUri::parse(&format!("nex://{}", actor_hex)).is_err());
}

#[test]
fn test_r58_1_b_nex_uri_resolver_to_smt_object() {
    let dir = tempdir().unwrap();
    let seed = [51u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns = [0x55u8; 32];
    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), "/photos/vacation.jpg".to_string());
    let payload = b"Binary JPEG Payload Bytes";

    let obj_id = node.create_object(ns, ObjectType::DriveInode, meta, payload.to_vec()).unwrap();
    assert_ne!(obj_id, [0u8; 32]);

    let uri_str = format!("nex://{}/{}/photos/vacation.jpg", hex::encode(node.identity.actor_id), hex::encode(ns));
    let uri = NexUri::parse(&uri_str).unwrap();

    let resolved = NexUriResolver::resolve_uri(&node, &uri);
    assert!(resolved.is_some());
    let obj = resolved.unwrap();
    assert_eq!(obj.payload_bytes, payload);
}

#[test]
fn test_r58_1_c_spatial_geopoint_and_geohash() {
    let pt1 = SpatialGeoPoint::new(37.7749, -122.4194); // San Francisco
    let pt2 = SpatialGeoPoint::new(37.7749, -122.4194);
    assert_eq!(pt1.geohash, pt2.geohash, "Identical coordinates must produce identical GeoHash");

    let pt3 = SpatialGeoPoint::new(40.7128, -74.0060); // New York
    assert_ne!(pt1.geohash, pt3.geohash, "Different coordinates must produce different GeoHash");
}

#[test]
fn test_r58_1_d_spatial_map_engine_bounding_box_query() {
    let dir = tempdir().unwrap();
    let seed = [52u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns_maps = [0xAA; 32];

    // POI 1: San Francisco (37.7749, -122.4194)
    let mut meta1 = BTreeMap::new();
    meta1.insert("name".to_string(), "Golden Gate".to_string());
    meta1.insert("lat".to_string(), "37.7749".to_string());
    meta1.insert("lon".to_string(), "-122.4194".to_string());
    node.create_object(ns_maps, ObjectType::Synthetic(10), meta1, b"SF POI".to_vec()).unwrap();

    // POI 2: New York (40.7128, -74.0060)
    let mut meta2 = BTreeMap::new();
    meta2.insert("name".to_string(), "Central Park".to_string());
    meta2.insert("lat".to_string(), "40.7128".to_string());
    meta2.insert("lon".to_string(), "-74.0060".to_string());
    node.create_object(ns_maps, ObjectType::Synthetic(10), meta2, b"NYC POI".to_vec()).unwrap();

    // Query California bounding box: lat (35.0..40.0), lon (-125.0..-120.0)
    let sf_results = SpatialMapEngine::query_bounding_box(&node, 35.0, 40.0, -125.0, -120.0);
    assert_eq!(sf_results.len(), 1);
    assert_eq!(sf_results[0].metadata.get("name").unwrap(), "Golden Gate");

    // Query New York bounding box: lat (40.0..42.0), lon (-75.0..-73.0)
    let nyc_results = SpatialMapEngine::query_bounding_box(&node, 40.0, 42.0, -75.0, -73.0);
    assert_eq!(nyc_results.len(), 1);
    assert_eq!(nyc_results[0].metadata.get("name").unwrap(), "Central Park");
}

#[test]
fn test_r58_1_e_group_federation_capability_token() {
    let group_root_seed = [53u8; 32];
    let group_root = SigningKey::from_bytes(&group_root_seed);

    let member_seed = [54u8; 32];
    let member_key = SigningKey::from_bytes(&member_seed);
    let member_actor = derive_actor_id(KeyType::Ed25519, &member_key.verifying_key().to_bytes());

    let group_id = [0x99u8; 32];
    let proof = GroupFederationEngine::create_group_capability_token(
        &group_root,
        member_actor,
        group_id,
        OP_READ | OP_WRITE,
    );

    assert_eq!(proof.token.subject, member_actor);
    assert_eq!(proof.token.namespace, group_id);
    assert!(proof.issuer_pubkey.is_some());
}

#[test]
fn test_r58_1_f_zero_regression_across_app_platform_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [55u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    assert_eq!(node.schema_version, 1);
    node.stop().unwrap();
}
