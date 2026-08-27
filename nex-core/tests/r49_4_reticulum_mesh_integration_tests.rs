use nex_core::transport::adapter::{
    TransportAdapter, ReticulumNativeAdapter, derive_reticulum_destination_hash
};
use nex_core::transport::dispatcher::MultiTransportDispatcher;
use nex_core::transport::types::TransportError;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::types::KeyType;

#[test]
fn test_r49_4_a_rns_destination_derivation() {
    let actor_a = derive_actor_id(KeyType::Ed25519, &[0xAA; 32]);
    let actor_b = derive_actor_id(KeyType::Ed25519, &[0xBB; 32]);

    let dest_a = derive_reticulum_destination_hash(&actor_a);
    let dest_b = derive_reticulum_destination_hash(&actor_b);

    assert_eq!(dest_a.len(), 16, "RNS destination hash must be exactly 16 bytes");
    assert_eq!(dest_b.len(), 16);
    assert_ne!(dest_a, dest_b, "Distinct ActorIDs must derive distinct RNS destination hashes");

    // Determinism test
    let dest_a_repeat = derive_reticulum_destination_hash(&actor_a);
    assert_eq!(dest_a, dest_a_repeat, "RNS destination derivation must be 100% deterministic");
}

#[test]
fn test_r49_4_b_packet_mtu_enforcement_and_chunking() {
    let dest = [0x55; 16];
    let mut adapter = ReticulumNativeAdapter::new([0x11; 16]);

    // 2KB mutation frame
    let payload = vec![0x7A; 2048];
    adapter.send(&dest, &payload).expect("Send across Reticulum adapter must succeed");

    // Outbox must contain fragmented packets fitting within link MTU (500 bytes)
    assert!(!adapter.outbox.is_empty());
    for (target_dest, packet) in &adapter.outbox {
        assert_eq!(target_dest, &dest);
        assert!(packet.len() <= 500, "Every Reticulum packet must strictly respect 500-byte MTU");
        assert!(packet.len() >= 36, "Every chunk packet must contain at least 36-byte chunk header");
    }

    // 2048 bytes + 13 bytes wire header = 2061 bytes.
    // Chunk capacity = 500 - 36 = 464 bytes.
    // (2061 + 463) / 464 = 5 chunks.
    assert_eq!(adapter.outbox.len(), 5, "2048-byte payload must be split into exactly 5 Reticulum chunks");
}

#[test]
fn test_r49_4_c_mesh_multi_hop_forwarding_and_invariance() {
    let dest_a = [0x0A; 16];
    let dest_b = [0x0B; 16];

    let mut node_a = ReticulumNativeAdapter::new(dest_a);
    let mut node_b = ReticulumNativeAdapter::new(dest_b);

    let original_payload = b"SOVEREIGN_MESH_MUTATION_MULTI_HOP_PAYLOAD";

    // 1. Node A sends to Node B
    node_a.send(&dest_b, original_payload).unwrap();

    // 2. Simulated Multi-Hop Relay: Relay receives raw RF chunks and forwards to Node B
    let mut relay_forward_queue = Vec::new();
    while let Some((target, chunk)) = node_a.outbox.pop_front() {
        // Relay preserves chunk bit-for-bit
        relay_forward_queue.push((target, chunk));
    }

    // 3. Node B receives all relayed chunks
    for (target, chunk) in relay_forward_queue {
        assert_eq!(target, dest_b);
        node_b.ingest_packet(&dest_a, &chunk, 1).unwrap();
    }

    // 4. Node B polls reconstructed packet
    let received_pkt = node_b.poll_incoming().expect("Node B must reconstruct multi-hop packet");
    assert_eq!(received_pkt.transport_tag, 0x01);
    assert_eq!(received_pkt.source_address, dest_a);
    assert_eq!(received_pkt.payload, original_payload, "Mesh multi-hop transmission must preserve payload bit-for-bit");
}

#[test]
fn test_r49_4_d_out_of_order_packet_reassembly() {
    let dest_b = [0x0B; 16];
    let mut node_a = ReticulumNativeAdapter::new([0x0A; 16]);
    let mut node_b = ReticulumNativeAdapter::new(dest_b);

    let payload = vec![0x33u8; 1500]; // Multi-chunk payload
    node_a.send(&dest_b, &payload).unwrap();

    let mut chunks: Vec<_> = node_a.outbox.drain(..).collect();
    assert!(chunks.len() > 1);

    // Permute chunks into non-sequential order (e.g. reverse order)
    chunks.reverse();

    // Ingest chunks out of order into Node B
    for (target, chunk) in chunks {
        node_b.ingest_packet(&[0x0A; 16], &chunk, 1).unwrap();
    }

    let reassembled_pkt = node_b.poll_incoming().expect("Out-of-order packets must deterministically reassemble");
    assert_eq!(reassembled_pkt.payload, payload, "Out-of-order reassembly must match bit-for-bit");
}

#[test]
fn test_r49_4_e_stale_in_flight_stream_pruning() {
    let mut node = ReticulumNativeAdapter::new([0x01; 16]);

    // Send 10 partial chunk streams at Epoch 5 (total_chunks = 3, only send index 0)
    for i in 0..10 {
        let mut msg_id = [0u8; 32];
        msg_id[0] = i as u8;
        let mut partial_chunk = Vec::new();
        partial_chunk.extend_from_slice(&msg_id);
        partial_chunk.extend_from_slice(&0u16.to_be_bytes()); // index 0
        partial_chunk.extend_from_slice(&3u16.to_be_bytes()); // total 3
        partial_chunk.extend_from_slice(b"PARTIAL_CHUNK_DATA");

        node.ingest_packet(&[0x99; 16], &partial_chunk, 5).unwrap();
    }

    assert_eq!(node.reassembler.in_flight.len(), 10);

    // Advance to Epoch 50 (> 30 epoch TTL) and prune
    let pruned = node.reassembler.prune_stale_streams(50, 30);
    assert_eq!(pruned, 10, "All 10 stale in-flight streams must be pruned");
    assert_eq!(node.reassembler.in_flight.len(), 0, "Reassembly buffer must be completely clean");
}

#[test]
fn test_r49_4_f_dual_transport_failover() {
    let mut dispatcher = MultiTransportDispatcher::new();

    // Register Reticulum adapter (Tag 0x01)
    let ret_adapter = ReticulumNativeAdapter::new([0x11; 16]);
    dispatcher.register_adapter(Box::new(ret_adapter));

    // Register Mock QUIC adapter (Tag 0x02, disconnected)
    let mut quic_adapter = nex_core::transport::adapter::MockQuicAdapter::new();
    quic_adapter.connected = false; // Offline!
    dispatcher.register_adapter(Box::new(quic_adapter));

    let dest = b"rns_destination_hash";
    let payload = b"SOVEREIGN_FAILOVER_PAYLOAD";

    // Dispatching to offline QUIC with allow_failover: true fails over to Reticulum (Tag 0x01)
    let used_tag = dispatcher.dispatch(0x02, dest, payload, true).expect("Failover to Reticulum must succeed");
    assert_eq!(used_tag, 0x01, "Dispatcher must seamlessly failover to Reticulum when primary link is offline");

    // Dispatching with allow_failover: false returns NoRoutableTransport
    let fail_res = dispatcher.dispatch(0x02, dest, payload, false);
    assert_eq!(fail_res, Err(TransportError::NoRoutableTransport));
}
