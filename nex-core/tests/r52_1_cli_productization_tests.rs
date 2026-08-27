use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::ipc::rpc::NexRpcServer;
use nex_core::ipc::client::NexRpcClient;
use nex_core::cli::{NexCli, CliCommand};

fn setup_daemon() -> (Arc<Mutex<NexNode>>, String, thread::JoinHandle<()>, Arc<std::sync::atomic::AtomicBool>) {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();
    let node_arc = Arc::new(Mutex::new(node));

    let server = NexRpcServer::bind("127.0.0.1:0", node_arc.clone()).unwrap();
    let addr = server.local_addr().unwrap().to_string();

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

#[test]
fn test_r52_1_a_cli_ping_and_status_over_ipc() {
    let (_node, addr, handle, running) = setup_daemon();
    let client = NexRpcClient::new(&addr);

    // 1. Ping
    let ping_cmd = NexCli::parse_args(&["ping".to_string(), "--socket".to_string(), addr.clone()]);
    let (code, out) = NexCli::execute_client(&ping_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("PONG [Actor:"));

    // 2. Status
    let status_cmd = NexCli::parse_args(&["status".to_string(), "--socket".to_string(), addr]);
    let (code, out) = NexCli::execute_client(&status_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("objects_count"));

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r52_1_b_cli_drive_put_and_list_over_ipc() {
    let (_node, addr, handle, running) = setup_daemon();
    let client = NexRpcClient::new(&addr);

    let tmp = tempdir().unwrap();
    let test_file = tmp.path().join("whitepaper.txt");
    fs::write(&test_file, b"Sovereign Decentralized Data Platform").unwrap();

    // 1. Drive Put
    let put_cmd = NexCli::parse_args(&[
        "drive".to_string(),
        "put".to_string(),
        test_file.to_string_lossy().to_string(),
        "/docs/whitepaper.txt".to_string(),
        "--socket".to_string(),
        addr.clone(),
    ]);
    let (code, out) = NexCli::execute_client(&put_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("File uploaded to Drive: /docs/whitepaper.txt"));

    // 2. Drive List
    let ls_cmd = NexCli::parse_args(&[
        "drive".to_string(),
        "ls".to_string(),
        "/docs".to_string(),
        "--socket".to_string(),
        addr,
    ]);
    let (code, out) = NexCli::execute_client(&ls_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("/docs/whitepaper.txt"));

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r52_1_c_cli_chat_send_over_ipc() {
    let (_node, addr, handle, running) = setup_daemon();
    let client = NexRpcClient::new(&addr);

    let chat_cmd = NexCli::parse_args(&[
        "chat".to_string(),
        "send".to_string(),
        "0101010101010101010101010101010101010101010101010101010101010101".to_string(),
        "Hello".to_string(),
        "from".to_string(),
        "CLI!".to_string(),
        "--socket".to_string(),
        addr,
    ]);
    let (code, out) = NexCli::execute_client(&chat_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("Message sent (ObjectID:"));

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r52_1_d_cli_community_post_over_ipc() {
    let (_node, addr, handle, running) = setup_daemon();
    let client = NexRpcClient::new(&addr);

    let post_cmd = NexCli::parse_args(&[
        "community".to_string(),
        "post".to_string(),
        "0202020202020202020202020202020202020202020202020202020202020202".to_string(),
        "Welcome".to_string(),
        "This is a community post over socket IPC.".to_string(),
        "--socket".to_string(),
        addr,
    ]);
    let (code, out) = NexCli::execute_client(&post_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("Post published: 'Welcome'"));

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r52_1_e_cli_remote_sync_and_gc_over_ipc() {
    let (_node, addr, handle, running) = setup_daemon();
    let client = NexRpcClient::new(&addr);

    // 1. Sync
    let sync_cmd = NexCli::parse_args(&["sync".to_string(), "--socket".to_string(), addr.clone()]);
    let (code, out) = NexCli::execute_client(&sync_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("state_root"));

    // 2. GC
    let gc_cmd = NexCli::parse_args(&["gc".to_string(), "--socket".to_string(), addr]);
    let (code, out) = NexCli::execute_client(&gc_cmd, &client);
    assert_eq!(code, 0);
    assert!(out.contains("Reclaimed 0 unreachable CAS chunks"));

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn test_r52_1_f_zero_regression_across_all_cli_application_flows() {
    let (node, addr, handle, running) = setup_daemon();
    let client = NexRpcClient::new(&addr);

    // Sequence: status (0) -> drive put -> chat send -> community post -> sync -> status (3)
    let s1 = NexCli::execute_client(&NexCli::parse_args(&["status".to_string(), "--socket".to_string(), addr.clone()]), &client);
    assert_eq!(s1.0, 0);
    assert!(s1.1.contains("\"objects_count\": 0"));

    let tmp = tempdir().unwrap();
    let file = tmp.path().join("test.bin");
    fs::write(&file, vec![0xCA, 0xFE, 0xBA, 0xBE]).unwrap();

    let d = NexCli::execute_client(&NexCli::parse_args(&[
        "drive".to_string(), "put".to_string(), file.to_string_lossy().to_string(), "/bin/test.bin".to_string(), "--socket".to_string(), addr.clone()
    ]), &client);
    assert_eq!(d.0, 0);

    let c = NexCli::execute_client(&NexCli::parse_args(&[
        "chat".to_string(), "send".to_string(), "00".repeat(32), "hi".to_string(), "--socket".to_string(), addr.clone()
    ]), &client);
    assert_eq!(c.0, 0);

    let comm = NexCli::execute_client(&NexCli::parse_args(&[
        "community".to_string(), "post".to_string(), "00".repeat(32), "Title".to_string(), "Body".to_string(), "--socket".to_string(), addr.clone()
    ]), &client);
    assert_eq!(comm.0, 0);

    let sync_res = NexCli::execute_client(&NexCli::parse_args(&["sync".to_string(), "--socket".to_string(), addr.clone()]), &client);
    assert_eq!(sync_res.0, 0);

    let s2 = NexCli::execute_client(&NexCli::parse_args(&["status".to_string(), "--socket".to_string(), addr]), &client);
    assert_eq!(s2.0, 0);
    assert!(s2.1.contains("\"objects_count\": 3"));

    let guard = node.lock().unwrap();
    assert_eq!(guard.state.object_store.len(), 3);

    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}
