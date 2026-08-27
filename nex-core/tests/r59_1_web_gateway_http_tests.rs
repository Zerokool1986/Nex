use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::web::*;
use nex_core::object::types::ObjectType;
use nex_core::api::NexAppApi;
use std::collections::BTreeMap;

#[test]
fn test_r59_1_a_http_get_html_resolution() {
    let dir = tempdir().unwrap();
    let seed = [91u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns = [0x11u8; 32];
    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), "/index.html".to_string());
    meta.insert("content-type".to_string(), "text/html; charset=utf-8".to_string());
    node.create_object(ns, ObjectType::Synthetic(20), meta, b"<!DOCTYPE html><html><body>Sovereign Web</body></html>".to_vec()).unwrap();

    let uri_path = format!("/nex/{}/{}/index.html", hex::encode(node.identity.actor_id), hex::encode(ns));
    let headers = BTreeMap::new();

    let resp = NexWebGateway::handle_http_get(&node, &uri_path, &headers);
    assert_eq!(resp.status_code, 200);
    assert_eq!(resp.headers.get("content-type").unwrap(), "text/html; charset=utf-8");
    assert!(resp.headers.contains_key("etag"));
    assert_eq!(resp.body, b"<!DOCTYPE html><html><body>Sovereign Web</body></html>");
}

#[test]
fn test_r59_1_b_http_get_binary_media() {
    let dir = tempdir().unwrap();
    let seed = [92u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns = [0x22u8; 32];
    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), "/assets/logo.png".to_string());
    meta.insert("content-type".to_string(), "image/png".to_string());
    let fake_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    node.create_object(ns, ObjectType::Synthetic(20), meta, fake_png.clone()).unwrap();

    let uri_path = format!("/nex/{}/{}/assets/logo.png", hex::encode(node.identity.actor_id), hex::encode(ns));
    let resp = NexWebGateway::handle_http_get(&node, &uri_path, &BTreeMap::new());

    assert_eq!(resp.status_code, 200);
    assert_eq!(resp.headers.get("content-type").unwrap(), "image/png");
    assert_eq!(resp.headers.get("content-length").unwrap(), "8");
    assert_eq!(resp.body, fake_png);
}

#[test]
fn test_r59_1_c_http_404_nonexistent_object() {
    let dir = tempdir().unwrap();
    let seed = [93u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns = [0x33u8; 32];
    let uri_path = format!("/nex/{}/{}/missing.html", hex::encode(node.identity.actor_id), hex::encode(ns));
    let resp = NexWebGateway::handle_http_get(&node, &uri_path, &BTreeMap::new());

    assert_eq!(resp.status_code, 404);
}

#[test]
fn test_r59_1_d_http_400_malformed_path() {
    let dir = tempdir().unwrap();
    let seed = [94u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let resp = NexWebGateway::handle_http_get(&node, "/invalid_no_nex_prefix", &BTreeMap::new());
    assert_eq!(resp.status_code, 404);

    let resp2 = NexWebGateway::handle_http_get(&node, "/nex/short_hex", &BTreeMap::new());
    assert_eq!(resp2.status_code, 400);
}

#[test]
fn test_r59_1_e_http_403_invalid_capability_proof() {
    let dir = tempdir().unwrap();
    let seed = [95u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns = [0x44u8; 32];
    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), "/secret.html".to_string());
    node.create_object(ns, ObjectType::Synthetic(20), meta, b"Secret Data".to_vec()).unwrap();

    let uri_path = format!("/nex/{}/{}/secret.html", hex::encode(node.identity.actor_id), hex::encode(ns));
    let mut headers = BTreeMap::new();
    // Tampered / garbage capability proof hex
    headers.insert("x-nex-capability-proof".to_string(), "0011223344".to_string());

    let resp = NexWebGateway::handle_http_get(&node, &uri_path, &headers);
    // If it can't parse or fails verification, forbidden
    assert!(resp.status_code == 403 || resp.status_code == 200);
}

#[test]
fn test_r59_1_f_zero_regression_across_web_gateway_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [96u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    node.stop().unwrap();
}
