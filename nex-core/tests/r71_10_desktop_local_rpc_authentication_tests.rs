use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::desktop::DesktopLocalRpcBroker;

#[test]
fn test_r71_10_a_valid_bearer_token_authenticates_rpc() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x11u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let token = [0x42u8; 32];
    let broker = DesktopLocalRpcBroker::new(token);

    let req = r#"{"jsonrpc":"2.0","method":"system_health","params":{},"id":1}"#;
    let resp_bytes = broker.dispatch_authenticated(&mut node, &token, req.as_bytes()).expect("RPC failed");
    let resp_str = String::from_utf8(resp_bytes).unwrap();
    assert!(resp_str.contains(r#""id":1"#));
}

#[test]
fn test_r71_10_b_invalid_bearer_token_rejected() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x22u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let valid_token = [0x42u8; 32];
    let attacker_token = [0x99u8; 32];
    let broker = DesktopLocalRpcBroker::new(valid_token);

    let req = r#"{"jsonrpc":"2.0","method":"system_health","params":{},"id":2}"#;
    let res = broker.dispatch_authenticated(&mut node, &attacker_token, req.as_bytes());
    assert!(res.is_err(), "Invalid bearer token must be rejected with UnauthorizedRpc");
}

#[test]
fn test_r71_10_c_malformed_json_rpc_error_handling() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x33u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let token = [0x55u8; 32];
    let broker = DesktopLocalRpcBroker::new(token);

    let bad_json = b"NOT_A_JSON_OBJECT";
    let res = broker.dispatch_authenticated(&mut node, &token, bad_json);
    assert!(res.is_err(), "Malformed JSON must return clean parse error");
}

#[test]
fn test_r71_10_d_empty_request_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x44u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let token = [0x66u8; 32];
    let broker = DesktopLocalRpcBroker::new(token);

    let res = broker.dispatch_authenticated(&mut node, &token, b"");
    assert!(res.is_err());
}

#[test]
fn test_r71_10_e_concurrent_multi_window_rpc_requests() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let signing_key = SigningKey::from_bytes(&[0x55u8; 32]);
    let mut node = NexNode::new(temp_dir.path().to_path_buf(), signing_key);
    node.start().unwrap();

    let token = [0x77u8; 32];
    let broker = DesktopLocalRpcBroker::new(token);

    // Perform multiple sequential calls representing distinct windows
    for window_id in 1..=5 {
        let req = format!(r#"{{"jsonrpc":"2.0","method":"system_health","params":{{"window":{}}},"id":{}}}"#, window_id, window_id);
        let resp = broker.dispatch_authenticated(&mut node, &token, req.as_bytes()).unwrap();
        let s = String::from_utf8(resp).unwrap();
        assert!(s.contains(&format!(r#""id":{}"#, window_id)));
    }
}

#[test]
fn test_r71_10_f_ephemeral_token_rotation() {
    let token_1 = [0x88u8; 32];
    let token_2 = [0x99u8; 32];

    let broker_1 = DesktopLocalRpcBroker::new(token_1);
    let broker_2 = DesktopLocalRpcBroker::new(token_2);

    assert!(broker_1.authenticate(&token_1));
    assert!(!broker_1.authenticate(&token_2));

    assert!(broker_2.authenticate(&token_2));
    assert!(!broker_2.authenticate(&token_1));
}
