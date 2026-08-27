use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};
use nex_core::runtime::production::NodeOperationalState;
use nex_core::runtime::mobile::DesktopPlatformAdapter;
use nex_core::api::NexAppApi;

#[test]
fn test_r57_2_a_desktop_loopback_rpc_server() {
    let dir = tempdir().unwrap();
    let seed = [201u8; 32];
    let signing_key = SigningKey::from_bytes(&seed);

    let mut node = NexNode::new(dir.path(), signing_key);
    assert!(node.start().is_ok());

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let req: JsonRpcRequest = serde_json::from_str(&line).unwrap();
        let resp = NexRpcDispatcher::dispatch_node(&mut node, req);
        let resp_bytes = serde_json::to_vec(&resp).unwrap();
        stream.write_all(&resp_bytes).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
    });

    let mut client = TcpStream::connect(addr).unwrap();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "nex_ping",
        "params": {}
    }).to_string();

    client.write_all(req.as_bytes()).unwrap();
    client.write_all(b"\n").unwrap();
    client.flush().unwrap();

    let mut reader = BufReader::new(client);
    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).unwrap();

    let resp_val: serde_json::Value = serde_json::from_str(&resp_line).unwrap();
    assert_eq!(resp_val["result"]["status"], "pong");

    server_handle.join().unwrap();
}

#[test]
fn test_r57_2_b_desktop_platform_adapter_health_poll() {
    let dir = tempdir().unwrap();
    let adapter = DesktopPlatformAdapter::new("nex-daemon", dir.path().to_path_buf());

    let seed = [202u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let health = adapter.poll_daemon_health(&node);
    assert!(health.is_ok());
    assert!(health.unwrap().contains("Running"));
}

#[test]
fn test_r57_2_c_desktop_pid_lockfile_exclusivity() {
    let dir = tempdir().unwrap();
    let seed1 = [203u8; 32];
    let mut node1 = NexNode::new(dir.path(), SigningKey::from_bytes(&seed1));
    assert!(node1.start().is_ok());

    // Second daemon on same directory must fail due to lockfile
    let seed2 = [204u8; 32];
    let mut node2 = NexNode::new(dir.path(), SigningKey::from_bytes(&seed2));
    assert!(node2.start().is_err(), "Second node on same path must fail");

    node1.stop().unwrap();
    // After node1 stops, node2 must acquire lock
    assert!(node2.start().is_ok());
}

#[test]
fn test_r57_2_d_scoped_app_data_directory_isolation() {
    let dir = tempdir().unwrap();
    let app_dir = dir.path().join("nex_desktop_app");
    std::fs::create_dir_all(&app_dir).unwrap();

    let seed = [205u8; 32];
    let mut node = NexNode::new(&app_dir, SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    assert_eq!(node.storage.data_dir, app_dir);
}

#[test]
fn test_r57_2_e_hot_reload_and_state_recovery() {
    let dir = tempdir().unwrap();
    let seed = [206u8; 32];

    {
        let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
        assert!(node.start().is_ok());
        let _ = node.sync_now();
        node.stop().unwrap();
    }

    {
        let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
        assert!(node.start().is_ok());
        assert_eq!(node.schema_version, 1);
    }
}

#[test]
fn test_r57_2_f_zero_regression_desktop_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [207u8; 32];

    for _ in 0..3 {
        let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
        assert!(node.start().is_ok());
        assert_eq!(node.operational_state, NodeOperationalState::Running);
        node.stop().unwrap();
    }
}
