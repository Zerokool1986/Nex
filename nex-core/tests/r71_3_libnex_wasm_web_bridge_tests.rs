use nex_core::apps::web::{NexWebGateway, WebAppManifest, WebRtcNaspBridge};
use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;

#[test]
fn test_r71_3_a_webapp_manifest_default_secure() {
    let manifest = WebAppManifest::default_secure("org.nex.drive", "NEX Drive");
    assert_eq!(manifest.app_id, "org.nex.drive");
    assert_eq!(manifest.name, "NEX Drive");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.entrypoint, "/index.html");
    assert!(manifest.content_security_policy.contains("default-src 'self' nex:;"));
}

#[test]
fn test_r71_3_b_webrtc_nasp_bridge_framing_roundtrip() {
    let payload = b"NEX/FRAME/v1/E2EE_ENCRYPTED_AEAD_PAYLOAD_TEST";
    let framed = WebRtcNaspBridge::frame_data_channel_message(payload);
    assert_eq!(framed.len(), 4 + payload.len());

    let unframed = WebRtcNaspBridge::unframe_data_channel_message(&framed).expect("Unframing failed");
    assert_eq!(unframed, payload);
}

#[test]
fn test_r71_3_c_webrtc_nasp_incomplete_frame_rejection() {
    let short_data = [0x00, 0x00];
    let res = WebRtcNaspBridge::unframe_data_channel_message(&short_data);
    assert!(res.is_err());

    let truncated_frame = [0x00, 0x00, 0x00, 0x10, 0xAA, 0xBB];
    let res_trunc = WebRtcNaspBridge::unframe_data_channel_message(&truncated_frame);
    assert!(res_trunc.is_err());
}

#[test]
fn test_r71_3_d_http_gateway_not_found_on_empty_route() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x11u8; 32]);
    let node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);

    let headers = BTreeMap::new();
    let resp = NexWebGateway::handle_http_get(&node, "/invalid/path", &headers);
    assert_eq!(resp.status_code, 404);
}

#[test]
fn test_r71_3_e_http_gateway_malformed_nex_uri_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x22u8; 32]);
    let node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);

    let headers = BTreeMap::new();
    let resp = NexWebGateway::handle_http_get(&node, "/nex/malformed_hex", &headers);
    assert_eq!(resp.status_code, 400);
}

#[test]
fn test_r71_3_f_custom_webapp_manifest_csp_integrity() {
    let manifest = WebAppManifest {
        app_id: "org.nex.vault".into(),
        name: "NEX Vault".into(),
        version: "2.1.0".into(),
        entrypoint: "/app.html".into(),
        content_security_policy: "default-src 'self' nex:; connect-src 'none';".into(),
    };

    assert_eq!(manifest.app_id, "org.nex.vault");
    assert!(manifest.content_security_policy.contains("connect-src 'none'"));
}
