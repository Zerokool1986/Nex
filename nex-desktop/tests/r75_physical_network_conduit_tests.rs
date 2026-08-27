use std::collections::BTreeMap;
use std::net::SocketAddr;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

use nex_core::runtime::node::NexNode;
use nex_core::object::types::{NexObject, ObjectType};
use nex_core::transport::socket::{LanTcpTransportServer, LanTcpTransportClient};
use nex_core::discovery::beacon::{DiscoveryBeacon, LanBeaconService, DiscoveredPeer};

use nex_desktop::app::{NexDesktopApp, AppStatus, NetworkTelemetry};
use nex_desktop::ui::NexUiState;

fn create_test_desktop_app(node_name: &str) -> (NexDesktopApp, tempfile::TempDir) {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let transport_server = LanTcpTransportServer::bind("127.0.0.1:0").ok();
    let tcp_bind_addr = transport_server.as_ref().map(|s| s.bind_addr);
    let tcp_port = tcp_bind_addr.map(|a| a.port()).unwrap_or(0);

    let beacon = DiscoveryBeacon::new(
        node.identity.actor_id,
        tcp_port,
        [0x77; 32],
        node_name,
    );

    let beacon_service = LanBeaconService::bind(beacon, 0, 0).ok();

    let network_telemetry = NetworkTelemetry {
        bytes_sent: 0,
        bytes_received: 0,
        active_conduits: 0,
        tcp_bind_addr,
        last_sync_epoch: 0,
        peer_sync_success_count: 0,
    };

    let app = NexDesktopApp {
        node,
        data_dir: tmp.path().to_path_buf(),
        ui: NexUiState::new(),
        status: AppStatus::Running,
        transport_server,
        beacon_service,
        network_telemetry,
        discovered_peers: Vec::new(),
        recovery_plan: None,
        recovery_shares: Vec::new(),
        active_crl: std::collections::BTreeSet::new(),
    };

    (app, tmp)
}

#[test]
fn test_physical_tcp_socket_server_bind_and_poll() {
    let (mut app, _tmp) = create_test_desktop_app("Node A");

    assert!(app.transport_server.is_some(), "Transport server must bind on dynamic loopback port");
    let bind_addr = app.network_telemetry.tcp_bind_addr.unwrap();
    assert_ne!(bind_addr.port(), 0);

    // Polling empty socket must return None without error
    let poll_res = app.transport_server.as_ref().unwrap().poll_and_sync_one(&mut app.node);
    assert!(poll_res.is_ok());
    assert_eq!(poll_res.unwrap(), None);
}

#[test]
fn test_physical_udp_discovery_beacon_serialization_and_parsing() {
    let actor_id = [0x42; 32];
    let beacon = DiscoveryBeacon::new(actor_id, 8080, [0x99; 32], "Laptop Node");

    let bytes = beacon.serialize().unwrap();
    let parsed = DiscoveryBeacon::deserialize(&bytes).unwrap();

    assert_eq!(parsed.actor_id, actor_id);
    assert_eq!(parsed.tcp_port, 8080);
    assert_eq!(parsed.node_name, "Laptop Node");
    assert_eq!(parsed.blinded_topic, [0x99; 32]);
}

#[test]
fn test_physical_cross_node_socket_sync_and_object_replication() {
    let (mut node_a_app, _tmp_a) = create_test_desktop_app("Node A");
    let (mut node_b_app, _tmp_b) = create_test_desktop_app("Node B");

    let server_a_addr = node_a_app.network_telemetry.tcp_bind_addr.unwrap();

    // 1. Ingest Photo into Node A
    let photo_id = [0x51; 32];
    let mut photo_meta = BTreeMap::new();
    photo_meta.insert("title".to_string(), "Physical Wire Sunset".to_string());
    photo_meta.insert("mime".to_string(), "image/jpeg".to_string());
    photo_meta.insert("space".to_string(), "Family".to_string());

    node_a_app.node.state.object_store.insert(photo_id, NexObject {
        object_id: photo_id,
        namespace: [0xFA; 32],
        object_type: ObjectType::PhotoMedia,
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 5,
        owner_actor_id: node_a_app.node.identity.actor_id,
        winning_mutation_id: [0u8; 32],
        metadata: photo_meta,
        payload_bytes: vec![0xAB; 2048],
        tombstoned: false,
    });

    // 2. Ingest Document into Node A
    let doc_id = [0x52; 32];
    let mut doc_meta = BTreeMap::new();
    doc_meta.insert("filename".to_string(), "Sovereignty_Charter.pdf".to_string());
    doc_meta.insert("mime".to_string(), "application/pdf".to_string());
    doc_meta.insert("space".to_string(), "Family".to_string());

    node_a_app.node.state.object_store.insert(doc_id, NexObject {
        object_id: doc_id,
        namespace: [0xFA; 32],
        object_type: ObjectType::DriveInode,
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 6,
        owner_actor_id: node_a_app.node.identity.actor_id,
        winning_mutation_id: [0u8; 32],
        metadata: doc_meta,
        payload_bytes: vec![0xCD; 4096],
        tombstoned: false,
    });

    // 3. Node B connects to Node A over real TCP stream and executes anti-entropy sync
    let sync_handle = std::thread::spawn(move || {
        // Run server A accept
        let mut attempts = 0;
        loop {
            if let Ok(Some(_)) = node_a_app.transport_server.as_ref().unwrap().poll_and_sync_one(&mut node_a_app.node) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            attempts += 1;
            if attempts > 40 { break; }
        }
        node_a_app
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let client_sync_res = LanTcpTransportClient::sync_with_remote_node(&mut node_b_app.node, server_a_addr);
    assert!(client_sync_res.is_ok(), "Client sync over real TCP socket must succeed: {:?}", client_sync_res);

    let _node_a_app = sync_handle.join().unwrap();

    // 4. Verify Node B now contains Node A's replicated objects bit-for-bit
    assert!(node_b_app.node.state.object_store.contains_key(&photo_id));
    assert!(node_b_app.node.state.object_store.contains_key(&doc_id));

    let replicated_photo = node_b_app.node.state.object_store.get(&photo_id).unwrap();
    assert_eq!(replicated_photo.metadata.get("title").unwrap(), "Physical Wire Sunset");
    assert_eq!(replicated_photo.payload_bytes, vec![0xAB; 2048]);

    let replicated_doc = node_b_app.node.state.object_store.get(&doc_id).unwrap();
    assert_eq!(replicated_doc.metadata.get("filename").unwrap(), "Sovereignty_Charter.pdf");
    assert_eq!(replicated_doc.payload_bytes, vec![0xCD; 4096]);
}

#[test]
fn test_physical_conduit_topology_radar_integration() {
    let (mut app, _tmp) = create_test_desktop_app("Local Host");

    let peer_actor = [0x88; 32];
    let peer_addr: SocketAddr = "127.0.0.1:45678".parse().unwrap();

    // Manually register a discovered physical peer
    app.discovered_peers.push(DiscoveredPeer {
        actor_id: peer_actor,
        addr: peer_addr,
        tcp_sync_addr: peer_addr,
        blinded_topic: [0x11; 32],
        last_seen_epoch: 100,
        node_name: "Amy's Linux Workstation".to_string(),
    });
    app.network_telemetry.active_conduits = 1;
    app.network_telemetry.bytes_received = 8192;

    // Verify Topology Radar builds dynamic node and conduit
    let (nodes, edges) = nex_desktop::ui::network::derive_topology(&app);

    let dynamic_node = nodes.iter().find(|n| n.label == "Amy's Linux Workstation");
    assert!(dynamic_node.is_some(), "Topology Radar must include discovered physical peer node");

    let dynamic_edge = edges.iter().find(|e| e.label.contains("Amy's Linux Workstation"));
    assert!(dynamic_edge.is_some(), "Topology Radar must include live wire conduit for physical peer");
    assert_eq!(dynamic_edge.unwrap().status, nex_desktop::ui::network::ConduitStatus::AvailableDirectMesh);
}
