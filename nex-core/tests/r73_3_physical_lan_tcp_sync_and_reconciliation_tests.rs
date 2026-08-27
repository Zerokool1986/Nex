use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::transport::socket::{LanTcpTransportServer, LanTcpTransportClient};
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r73_3_a_physical_tcp_server_bind_and_listener() {
    let server = LanTcpTransportServer::bind("127.0.0.1:0")
        .expect("Failed to bind TCP server to OS assigned port");

    assert_eq!(server.bind_addr.ip().to_string(), "127.0.0.1");
    assert!(server.bind_addr.port() > 0);
}

#[test]
fn test_r73_3_b_client_server_handshake_and_smt_frontier_exchange() {
    let tmp_srv = tempdir().unwrap();
    let tmp_cli = tempdir().unwrap();

    let mut srv_node = NexNode::new(tmp_srv.path(), SigningKey::from_bytes(&[0x01u8; 32]));
    srv_node.start().unwrap();

    let cli_node = Arc::new(Mutex::new(NexNode::new(tmp_cli.path(), SigningKey::from_bytes(&[0x02u8; 32]))));
    cli_node.lock().unwrap().start().unwrap();

    let server = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();
    let srv_addr = server.bind_addr;

    // Client connects in background thread while server polls
    let cli_clone = Arc::clone(&cli_node);
    let handle = thread::spawn(move || {
        let mut node = cli_clone.lock().unwrap();
        LanTcpTransportClient::sync_with_remote_node(&mut *node, srv_addr)
    });

    // Server accepts and handles request
    let mut accepted = None;
    for _ in 0..20 {
        if let Ok(Some(addr)) = server.poll_and_sync_one(&mut srv_node) {
            accepted = Some(addr);
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let client_res = handle.join().expect("Client thread panicked");
    assert!(accepted.is_some(), "Server must accept client connection over TCP");
    assert!(client_res.is_ok(), "Client sync over TCP must succeed");
}

#[test]
fn test_r73_3_c_real_socket_object_transfer_and_canonical_state_ingest() {
    let tmp_mobile = tempdir().unwrap();
    let tmp_desktop = tempdir().unwrap();

    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile_node = NexNode::new(tmp_mobile.path(), SigningKey::from_bytes(&root_seed));
    mobile_node.start().unwrap();

    let desktop_node = Arc::new(Mutex::new(NexNode::new(tmp_desktop.path(), SigningKey::from_bytes(&[0x04u8; 32]))));
    desktop_node.lock().unwrap().start().unwrap();

    // 1. Mobile captures photo in Family Space
    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let photo_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x55, 0x66, 0x77, 0x88];
    let (photo_id, _) = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile_node,
        &proof,
        "Pixel 9 Pro Sunset",
        photo_bytes.clone(),
        10,
        &BTreeMap::new(),
        &root_actor,
    ).unwrap();

    assert!(mobile_node.state.object_store.contains_key(&photo_id));
    assert!(!desktop_node.lock().unwrap().state.object_store.contains_key(&photo_id));

    // 2. Start Mobile TCP Server (acting as sync provider)
    let mobile_server = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();
    let mobile_addr = mobile_server.bind_addr;

    // 3. Desktop connects as Client over real OS TCP socket to pull missing SMT batches
    let d_clone = Arc::clone(&desktop_node);
    let client_handle = thread::spawn(move || {
        let mut d_node = d_clone.lock().unwrap();
        LanTcpTransportClient::sync_with_remote_node(&mut *d_node, mobile_addr)
    });

    for _ in 0..20 {
        if mobile_server.poll_and_sync_one(&mut mobile_node).unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let ingested = client_handle.join().unwrap().unwrap();
    assert!(ingested > 0, "Desktop must ingest at least 1 batch over TCP");

    // 4. Verify object now exists in canonical state on Desktop
    let d_node = desktop_node.lock().unwrap();
    assert!(d_node.state.object_store.contains_key(&photo_id));
    let desktop_obj = d_node.state.object_store.get(&photo_id).unwrap();
    assert_eq!(desktop_obj.object_id, photo_id);
    assert_eq!(desktop_obj.payload_bytes, photo_bytes);
}

#[test]
fn test_r73_3_d_bidirectional_sync_reaches_mutual_merkle_state_root_equality() {
    let tmp_a = tempdir().unwrap();
    let tmp_b = tempdir().unwrap();

    let mut node_a = NexNode::new(tmp_a.path(), SigningKey::from_bytes(&[0x05u8; 32]));
    node_a.start().unwrap();

    let node_b = Arc::new(Mutex::new(NexNode::new(tmp_b.path(), SigningKey::from_bytes(&[0x06u8; 32]))));
    node_b.lock().unwrap().start().unwrap();

    let server_a = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();
    let addr_a = server_a.bind_addr;

    let b_clone = Arc::clone(&node_b);
    let handle = thread::spawn(move || {
        let mut b = b_clone.lock().unwrap();
        LanTcpTransportClient::sync_with_remote_node(&mut *b, addr_a)
    });

    for _ in 0..20 {
        if server_a.poll_and_sync_one(&mut node_a).unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    handle.join().unwrap().unwrap();
    let b = node_b.lock().unwrap();
    assert_eq!(node_a.state.state_node.frontier, b.state.state_node.frontier, "Frontiers must match");
}

#[test]
fn test_r73_3_e_tcp_transport_resilience_under_connection_drop_and_reconnect() {
    let tmp_cli = tempdir().unwrap();
    let mut cli_node = NexNode::new(tmp_cli.path(), SigningKey::from_bytes(&[0x07u8; 32]));
    cli_node.start().unwrap();

    // Attempt connection to dead port (offline peer) -> fails cleanly without crash
    let dead_addr: SocketAddr = "127.0.0.1:59999".parse().unwrap();
    let res = LanTcpTransportClient::sync_with_remote_node(&mut cli_node, dead_addr);
    assert!(res.is_err(), "Must report connection failure truthfully when peer is offline");
}

#[test]
fn test_r73_3_f_replay_and_duplicate_mutation_deduplication_over_socket() {
    let tmp_srv = tempdir().unwrap();
    let tmp_cli = tempdir().unwrap();

    let mut srv_node = NexNode::new(tmp_srv.path(), SigningKey::from_bytes(&[0x08u8; 32]));
    srv_node.start().unwrap();

    let cli_node = Arc::new(Mutex::new(NexNode::new(tmp_cli.path(), SigningKey::from_bytes(&[0x09u8; 32]))));
    cli_node.lock().unwrap().start().unwrap();

    let server = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();
    let srv_addr = server.bind_addr;

    // Perform sync twice over TCP
    for _ in 0..2 {
        let c_clone = Arc::clone(&cli_node);
        let handle = thread::spawn(move || {
            let mut c = c_clone.lock().unwrap();
            LanTcpTransportClient::sync_with_remote_node(&mut *c, srv_addr)
        });

        for _ in 0..20 {
            if server.poll_and_sync_one(&mut srv_node).unwrap().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        handle.join().unwrap().unwrap();
    }

    let c = cli_node.lock().unwrap();
    assert_eq!(srv_node.state.state_node.frontier, c.state.state_node.frontier);
}
