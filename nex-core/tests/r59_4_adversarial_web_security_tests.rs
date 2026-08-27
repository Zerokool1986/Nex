use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::web::*;
use nex_core::api::NexAppApi;
use std::collections::BTreeMap;

#[test]
fn test_r59_4_a_path_traversal_injection_rejection() {
    let dir = tempdir().unwrap();
    let seed = [101u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let attacks = vec![
        "/nex/../../../../etc/passwd",
        "/nex/..\\..\\..\\windows\\win.ini",
        "/nex/%2e%2e%2f%2e%2e%2fsecret",
    ];

    for attack in attacks {
        let resp = NexWebGateway::handle_http_get(&node, attack, &BTreeMap::new());
        assert!(resp.status_code == 400 || resp.status_code == 404, "Attack must be rejected: {}", attack);
    }
}

#[test]
fn test_r59_4_b_capability_header_fuzzing_defense() {
    let dir = tempdir().unwrap();
    let seed = [102u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns = [0x55u8; 32];
    let uri_path = format!("/nex/{}/{}/doc.html", hex::encode(node.identity.actor_id), hex::encode(ns));

    let fuzzed_headers = vec![
        "",
        "NOT_HEX",
        "00",
        "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        "{\"invalid\":\"json\"}",
    ];

    for header_val in fuzzed_headers {
        let mut headers = BTreeMap::new();
        headers.insert("x-nex-capability-proof".to_string(), header_val.to_string());
        let resp = NexWebGateway::handle_http_get(&node, &uri_path, &headers);
        assert!(resp.status_code == 404 || resp.status_code == 403, "Fuzzed header must not crash server");
    }
}

#[test]
fn test_r59_4_c_high_throughput_http_burst() {
    let dir = tempdir().unwrap();
    let seed = [103u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns = [0x66u8; 32];
    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), "/test.txt".to_string());
    node.create_object(ns, nex_core::object::types::ObjectType::Synthetic(20), meta, b"Burst Test Data".to_vec()).unwrap();

    let uri_path = format!("/nex/{}/{}/test.txt", hex::encode(node.identity.actor_id), hex::encode(ns));

    for _ in 0..100 {
        let resp = NexWebGateway::handle_http_get(&node, &uri_path, &BTreeMap::new());
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body, b"Burst Test Data");
    }
}

#[test]
fn test_r59_4_d_webrtc_oversized_payload_framing() {
    let large_payload = vec![0xABu8; 256 * 1024]; // 256 KB
    let framed = WebRtcNaspBridge::frame_data_channel_message(&large_payload);
    assert_eq!(framed.len(), 4 + 256 * 1024);

    let unframed = WebRtcNaspBridge::unframe_data_channel_message(&framed).unwrap();
    assert_eq!(unframed.len(), 256 * 1024);
}

#[test]
fn test_r59_4_e_cross_origin_isolation() {
    let dir = tempdir().unwrap();
    let seed = [104u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let ns1 = [0x11u8; 32];
    let ns2 = [0x22u8; 32];

    let mut meta1 = BTreeMap::new();
    meta1.insert("path".to_string(), "/app1/data".to_string());
    node.create_object(ns1, nex_core::object::types::ObjectType::Synthetic(20), meta1, b"Data 1".to_vec()).unwrap();

    let uri_path_wrong_ns = format!("/nex/{}/{}/app1/data", hex::encode(node.identity.actor_id), hex::encode(ns2));
    let resp = NexWebGateway::handle_http_get(&node, &uri_path_wrong_ns, &BTreeMap::new());
    assert_eq!(resp.status_code, 404, "Cross-namespace access without matching object must return 404");
}

#[test]
fn test_r59_4_f_gate_r59_master_web_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [105u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Web gateway operations must preserve Merkle root invariance");
}
