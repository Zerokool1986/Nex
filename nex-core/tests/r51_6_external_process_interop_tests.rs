use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde_json::Value;
use nex_core::runtime::node::NexNode;
use nex_core::ipc::rpc::{NexRpcServer, JsonRpcRequest, JsonRpcResponse};

fn setup_test_server() -> (Arc<Mutex<NexNode>>, std::net::SocketAddr, thread::JoinHandle<()>, Arc<std::sync::atomic::AtomicBool>) {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();
    let node_arc = Arc::new(Mutex::new(node));

    let server = NexRpcServer::bind("127.0.0.1:0", node_arc.clone()).unwrap();
    let addr = server.local_addr().unwrap();

    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();
    let node_clone = node_arc.clone();
    let listener = server.listener;
    listener.set_nonblocking(true).unwrap();

    let handle = thread::spawn(move || {
        while running_clone.load(std::sync::atomic::Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let n = node_clone.clone();
                    thread::spawn(move || {
                        NexRpcServer::handle_client(stream, n);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });

    (node_arc, addr, handle, running)
}

fn send_rpc_request(addr: std::net::SocketAddr, req_str: &str) -> Value {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream.write_all(req_str.as_bytes()).unwrap();
    stream.write_all(b"\n").unwrap();
    stream.flush().unwrap();

    let mut reader = BufReader::new(stream);
    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).unwrap();

    serde_json::from_str(&resp_line).expect("Must parse valid JSON-RPC response")
}

#[test]
fn test_r51_6_a_loopback_socket_rpc_server_boot_and_ping() {
    let (_node, addr, handle, running) = setup_test_server();

    let req = r#"{"jsonrpc": "2.0", "id": 1, "method": "nex_ping", "params": {}}"#;
    let resp = send_rpc_request(addr, req);

    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["status"], "pong");
    assert!(resp["result"]["actor_id"].is_string());

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r51_6_b_external_process_object_crud_over_socket_ipc() {
    let (_node, addr, handle, running) = setup_test_server();

    // 1. Create Object
    let create_req = r#"{"jsonrpc": "2.0", "id": 2, "method": "nex_createObject", "params": {"namespace": "0101010101010101010101010101010101010101010101010101010101010101", "payload": "68656c6c6f20697063", "metadata": {"author": "cli_user"}}}"#;
    let create_resp = send_rpc_request(addr, create_req);

    assert_eq!(create_resp["id"], 2);
    let obj_id = create_resp["result"]["object_id"].as_str().unwrap().to_string();
    assert!(!obj_id.is_empty());

    // 2. Read Object
    let read_req = format!(r#"{{"jsonrpc": "2.0", "id": 3, "method": "nex_readObject", "params": {{"object_id": "{}"}}}}"#, obj_id);
    let read_resp = send_rpc_request(addr, &read_req);

    assert_eq!(read_resp["id"], 3);
    assert_eq!(read_resp["result"]["object_id"], obj_id);
    assert_eq!(read_resp["result"]["tombstoned"], false);

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r51_6_c_concurrent_multi_client_socket_contention() {
    let (_node, addr, handle, running) = setup_test_server();

    let mut client_threads = Vec::new();
    for thread_idx in 0..10 {
        client_threads.push(thread::spawn(move || {
            for i in 0..10 {
                let req = format!(
                    r#"{{"jsonrpc": "2.0", "id": {}, "method": "nex_createObject", "params": {{"namespace": "0101010101010101010101010101010101010101010101010101010101010101", "payload": "deadbeef", "metadata": {{"t": "{}_{}"}}}}}}"#,
                    thread_idx * 100 + i,
                    thread_idx,
                    i
                );
                let resp = send_rpc_request(addr, &req);
                assert!(resp["result"]["object_id"].is_string());
            }
        }));
    }

    for t in client_threads {
        t.join().unwrap();
    }

    // Verify node has 100 objects
    let status_req = r#"{"jsonrpc": "2.0", "id": 999, "method": "nex_getStatus", "params": {}}"#;
    let status_resp = send_rpc_request(addr, status_req);
    assert_eq!(status_resp["result"]["objects_count"], 100);

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r51_6_d_client_abrupt_disconnect_handling() {
    let (_node, addr, handle, running) = setup_test_server();

    // Open connection, send partial bytes, and drop
    {
        let mut stream = TcpStream::connect(addr).unwrap();
        let _ = stream.write_all(b"{\"jsonrpc\": \"2.0\", ");
        // Abruptly drop stream
    }

    // Subsequent valid request must succeed immediately
    let req = r#"{"jsonrpc": "2.0", "id": 10, "method": "nex_ping", "params": {}}"#;
    let resp = send_rpc_request(addr, req);
    assert_eq!(resp["result"]["status"], "pong");

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r51_6_e_remote_sync_trigger_via_rpc() {
    let (_node, addr, handle, running) = setup_test_server();

    // Create 1 object
    let create_req = r#"{"jsonrpc": "2.0", "id": 1, "method": "nex_createObject", "params": {"namespace": "0101010101010101010101010101010101010101010101010101010101010101", "payload": "cafe", "metadata": {}}}"#;
    send_rpc_request(addr, create_req);

    // Remote syncNow trigger
    let sync_req = r#"{"jsonrpc": "2.0", "id": 2, "method": "nex_syncNow", "params": {}}"#;
    let sync_resp = send_rpc_request(addr, sync_req);

    assert_eq!(sync_resp["id"], 2);
    let root_hex = sync_resp["result"]["state_root"].as_str().unwrap();
    assert!(!root_hex.is_empty());
    assert_ne!(root_hex, "0000000000000000000000000000000000000000000000000000000000000000");

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r51_6_f_zero_regression_across_external_cli_lifecycle() {
    let (_node, addr, handle, running) = setup_test_server();

    // Full CLI flow: ping -> status -> create -> read -> sync -> status
    let r1 = send_rpc_request(addr, r#"{"jsonrpc": "2.0", "id": 1, "method": "nex_ping", "params": {}}"#);
    assert_eq!(r1["result"]["status"], "pong");

    let r2 = send_rpc_request(addr, r#"{"jsonrpc": "2.0", "id": 2, "method": "nex_getStatus", "params": {}}"#);
    assert_eq!(r2["result"]["objects_count"], 0);

    let r3 = send_rpc_request(addr, r#"{"jsonrpc": "2.0", "id": 3, "method": "nex_createObject", "params": {"namespace": "0202020202020202020202020202020202020202020202020202020202020202", "payload": "cli_lifecycle", "metadata": {}}}"#);
    let oid = r3["result"]["object_id"].as_str().unwrap();

    let r4 = send_rpc_request(addr, &format!(r#"{{"jsonrpc": "2.0", "id": 4, "method": "nex_readObject", "params": {{"object_id": "{}"}}}}"#, oid));
    assert_eq!(r4["result"]["tombstoned"], false);

    let r5 = send_rpc_request(addr, r#"{"jsonrpc": "2.0", "id": 5, "method": "nex_syncNow", "params": {}}"#);
    assert!(r5["result"]["state_root"].is_string());

    let r6 = send_rpc_request(addr, r#"{"jsonrpc": "2.0", "id": 6, "method": "nex_getStatus", "params": {}}"#);
    assert_eq!(r6["result"]["objects_count"], 1);

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}
