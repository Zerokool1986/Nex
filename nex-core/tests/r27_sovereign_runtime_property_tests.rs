use nex_core::runtime::node::SovereignNodeRuntime;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::types::KeyType;
use nex_core::transport::adapter::{MockQuicAdapter, MockReticulumAdapter};
use nex_core::transport::types::{encode_frame, TransportPacket};
use nex_core::sync::types::SyncMessage;
use nex_core::transport::fragmentation::fragment_payload;

#[test]
fn test_r27_a_sovereign_node_bootstrapping() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let namespace = [0xAA; 32];
    let image_id = [0x42; 32];

    let mut runtime = SovereignNodeRuntime::new(alice, namespace, image_id);
    runtime.transport.register_adapter(Box::new(MockQuicAdapter::new()));
    runtime.transport.register_adapter(Box::new(MockReticulumAdapter::new()));

    assert_eq!(runtime.actor_id, alice);
    assert_eq!(runtime.current_epoch, 0);

    let initial_cp = runtime.checkpoint();
    assert_eq!(initial_cp.body.frontier.len(), 0);
}

#[test]
fn test_r27_b_cross_node_application_sync_over_transport() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let namespace = [0xAA; 32];
    let image_id = [0x42; 32];

    let mut node_a = SovereignNodeRuntime::new(alice, namespace, image_id);
    let mut node_b = SovereignNodeRuntime::new(bob, namespace, image_id);

    // 1. Node A creates a Drive file and sends a Chat message
    let m_file = node_a.drive.create_file("/docs/manifesto.txt", [0x99; 32], 2048, "text/plain", 1);
    node_a.submit_local_mutation(m_file.clone());

    let channel_id = [0xCC; 32];
    let (_, m_chat) = node_a.chat.send_message(channel_id, alice, b"sovereign_runtime_online".to_vec(), 1);
    node_a.submit_local_mutation(m_chat.clone());

    let cp_a = node_a.checkpoint();

    // 2. Package mutations into SyncMessage envelopes and frame over transport
    let sync_file = SyncMessage::DirectMutationBroadcast(m_file);
    let sync_chat = SyncMessage::DirectMutationBroadcast(m_chat);

    let frame_file = encode_frame(0x02, 0x00, &serde_json::to_vec(&sync_file).unwrap());
    let frame_chat = encode_frame(0x02, 0x00, &serde_json::to_vec(&sync_chat).unwrap());

    // Chunk frames into low-MTU mesh chunks
    let chunk_file = fragment_payload([0x11; 32], &frame_file, 500).unwrap();
    let chunk_chat = fragment_payload([0x22; 32], &frame_chat, 500).unwrap();

    // 3. Deliver chunks to Node B's transport adapter inbox
    let mut mock_quic = MockQuicAdapter::new();
    for c in chunk_file {
        mock_quic.inbox.push_back(TransportPacket {
            transport_tag: 0x02,
            source_address: alice.to_vec(),
            payload: encode_frame(0x02, 0x00, &c),
        });
    }
    for c in chunk_chat {
        mock_quic.inbox.push_back(TransportPacket {
            transport_tag: 0x02,
            source_address: alice.to_vec(),
            payload: encode_frame(0x02, 0x00, &c),
        });
    }

    node_b.transport.register_adapter(Box::new(mock_quic));

    // 4. Node B executes tick() loop
    let dispositions = node_b.tick(1);
    assert_eq!(dispositions.len(), 2, "Node B must ingest and admit both application mutations");

    let cp_b = node_b.checkpoint();

    // Assert 100% byte-for-byte state convergence across nodes
    assert_eq!(cp_a.id, cp_b.id, "R27-B: Node A and Node B must converge to identical CheckpointID");
    assert_eq!(cp_a.body.state_root, cp_b.body.state_root);
    assert_eq!(cp_a.body.causal_root, cp_b.body.causal_root);
}

#[test]
fn test_r27_e_dynamic_carrier_failover_during_sync() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let namespace = [0xAA; 32];
    let image_id = [0x42; 32];

    let mut runtime = SovereignNodeRuntime::new(alice, namespace, image_id);

    let mut dead_quic = MockQuicAdapter::new();
    dead_quic.connected = false; // QUIC connection down
    runtime.transport.register_adapter(Box::new(dead_quic));

    let live_mesh = MockReticulumAdapter::new();
    runtime.transport.register_adapter(Box::new(live_mesh));

    // Dispatch to QUIC with allow_failover: true
    let used_tag = runtime.transport.dispatch(0x02, b"peer_addr", b"payload", true).expect("Failover must succeed");
    assert_eq!(used_tag, 0x01, "R27-E: Must automatically route over Reticulum mesh when QUIC is offline");
}

#[test]
fn test_r27_f_byzantine_injection_quarantining_in_runtime() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let attacker = derive_actor_id(KeyType::Ed25519, &[0xEE; 32]);
    let namespace = [0xAA; 32];
    let image_id = [0x42; 32];

    let mut runtime = SovereignNodeRuntime::new(alice, namespace, image_id);
    let mut quic = MockQuicAdapter::new();

    // 1. Attacker floods runtime with 100 corrupted CRC frames
    for _ in 0..100 {
        quic.inbox.push_back(TransportPacket {
            transport_tag: 0x02,
            source_address: attacker.to_vec(),
            payload: vec![0x4E, 0x58, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x04, 0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04], // Bad CRC
        });
    }

    runtime.transport.register_adapter(Box::new(quic));

    // Runtime processes tick() loop
    runtime.tick(1);

    // Attacker must be penalized and jailed!
    assert!(runtime.peer_jail.is_jailed(&attacker, 1), "R27-F: Flooding attacker must be jailed by runtime");
}
