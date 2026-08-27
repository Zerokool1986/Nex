use std::net::Shutdown;
use std::time::Duration;
use std::thread;
use nex_core::transport::adapter::{TransportAdapter, TcpTransportAdapter};
use nex_core::apps::drive::CasChunkStore;

#[test]
fn test_r49_3_a_real_socket_bind_and_listener_lifecycle() {
    let mut adapter = TcpTransportAdapter::bind("127.0.0.1:0").expect("Failed to bind physical loopback socket");
    assert!(adapter.local_addr.port() > 0, "OS must assign valid ephemeral port");
    assert!(adapter.is_connected());

    // Polling with no peers must return None without blocking
    let pkt = adapter.poll_incoming();
    assert!(pkt.is_none());
}

#[test]
fn test_r49_3_b_bidirectional_frame_transmission() {
    let mut node_a = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();
    let mut node_b = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();

    let addr_a = node_a.local_addr;
    let addr_b = node_b.local_addr;

    let payload_a_to_b = b"NEX_SYNC_PUSH_PAYLOAD_FROM_A";
    let payload_b_to_a = b"NEX_SYNC_ACK_PAYLOAD_FROM_B";

    // 1. Node A sends to Node B over real TCP
    node_a.send(addr_b.to_string().as_bytes(), payload_a_to_b).expect("Node A send must succeed");

    // Allow host TCP stack to process
    thread::sleep(Duration::from_millis(50));

    // 2. Node B polls and receives packet
    let pkt_b = node_b.poll_incoming().expect("Node B must receive incoming TCP packet");
    assert_eq!(pkt_b.transport_tag, 0x03);
    assert_eq!(pkt_b.payload, payload_a_to_b);

    // 3. Node B replies to Node A over real TCP
    node_b.send(addr_a.to_string().as_bytes(), payload_b_to_a).expect("Node B send must succeed");

    thread::sleep(Duration::from_millis(50));

    // 4. Node A polls and receives reply
    let pkt_a = node_a.poll_incoming().expect("Node A must receive reply TCP packet");
    assert_eq!(pkt_a.transport_tag, 0x03);
    assert_eq!(pkt_a.payload, payload_b_to_a);
}

#[test]
fn test_r49_3_c_bulk_high_throughput_cas_stream() {
    let mut sender = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();
    let mut receiver = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();

    let recv_addr = receiver.local_addr;

    // Create 10MB test payload
    let raw_payload = vec![0x42u8; 10 * 1024 * 1024];
    let mut sender_cas = CasChunkStore::new();
    let (content_root, chunk_digests) = sender_cas.store_file(&raw_payload);

    let mut receiver_cas = CasChunkStore::new();

    // Stream each 2MB chunk over physical TCP
    for digest in &chunk_digests {
        let chunk_data = sender_cas.get_chunk(digest).unwrap();
        sender.send(recv_addr.to_string().as_bytes(), chunk_data).expect("Chunk send must succeed");

        let mut pkt = None;
        for _ in 0..100 {
            if let Some(p) = receiver.poll_incoming() {
                pkt = Some(p);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let pkt = pkt.expect("Receiver must receive chunk packet");
        let recv_digest = receiver_cas.put_chunk(&pkt.payload);
        assert_eq!(&recv_digest, digest, "Received chunk digest must match sender SHA-256");
    }

    // Reassemble 10MB file on receiver
    let reassembled = receiver_cas.assemble_file(&chunk_digests).expect("Receiver reassembly must succeed");
    assert_eq!(reassembled.len(), raw_payload.len());
    assert_eq!(reassembled, raw_payload, "Reassembled 10MB payload must match bit-for-bit");
    assert_eq!(CasChunkStore::compute_merkle_root(&chunk_digests), content_root);
}

#[test]
fn test_r49_3_d_abrupt_socket_severance_and_reconnect() {
    let mut node_a = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();
    let mut node_b = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();

    let addr_b = node_b.local_addr;

    // Send initial frame
    node_a.send(addr_b.to_string().as_bytes(), b"FRAME_1").unwrap();
    thread::sleep(Duration::from_millis(30));
    let pkt1 = node_b.poll_incoming().unwrap();
    assert_eq!(pkt1.payload, b"FRAME_1");

    // Abrupt socket severance: shutdown all streams on Node A
    for stream in node_a.streams.values_mut() {
        let _ = stream.shutdown(Shutdown::Both);
    }
    node_a.streams.clear();

    // Reconnect and send second frame
    node_a.send(addr_b.to_string().as_bytes(), b"FRAME_2_AFTER_RECONNECT").expect("Send after reconnect must succeed");
    thread::sleep(Duration::from_millis(30));

    let pkt2 = node_b.poll_incoming().expect("Node B must receive packet after reconnect");
    assert_eq!(pkt2.payload, b"FRAME_2_AFTER_RECONNECT");
}

#[test]
fn test_r49_3_e_corrupted_wire_frame_defense() {
    use std::io::Write;
    use std::net::TcpStream;

    let mut receiver = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();
    let recv_addr = receiver.local_addr;

    // Connect raw client and send garbage/corrupt frames
    let mut raw_client = TcpStream::connect(recv_addr).unwrap();

    // 1. Invalid magic
    raw_client.write_all(&[0xFF, 0xFF, 0x03, 0x00, 0x00, 0x00, 0x04, 0x12, 0x34, 0x56, 0x78, 0xAA, 0xBB, 0xCC, 0xDD]).unwrap();
    raw_client.flush().unwrap();
    thread::sleep(Duration::from_millis(20));

    // Receiver must not crash and drop the corrupt frame
    assert!(receiver.poll_incoming().is_none());

    // 2. Corrupt CRC32 frame
    let mut valid_frame = nex_core::transport::types::encode_frame(0x03, 0x00, b"VALID");
    valid_frame[10] ^= 0xEE; // Invalidate CRC
    raw_client.write_all(&valid_frame).unwrap();
    raw_client.flush().unwrap();
    thread::sleep(Duration::from_millis(20));

    assert!(receiver.poll_incoming().is_none(), "Corrupt CRC32 frame must be dropped");

    // 3. Legitimate frame sent right after must succeed
    let clean_frame = nex_core::transport::types::encode_frame(0x03, 0x00, b"RECOVERED_VALID_FRAME");
    raw_client.write_all(&clean_frame).unwrap();
    raw_client.flush().unwrap();
    thread::sleep(Duration::from_millis(30));

    let valid_pkt = receiver.poll_incoming().expect("Clean frame after corruption must be received");
    assert_eq!(valid_pkt.payload, b"RECOVERED_VALID_FRAME");
}

#[test]
fn test_r49_3_f_concurrent_multi_client_load() {
    let mut server = TcpTransportAdapter::bind("127.0.0.1:0").unwrap();
    let server_addr = server.local_addr;

    let mut clients = Vec::new();
    for _ in 0..5 {
        clients.push(TcpTransportAdapter::bind("127.0.0.1:0").unwrap());
    }

    // 5 clients send 10 messages each
    for (client_idx, client) in clients.iter_mut().enumerate() {
        for msg_idx in 0..10 {
            let msg = format!("CLIENT_{}_MSG_{}", client_idx, msg_idx);
            client.send(server_addr.to_string().as_bytes(), msg.as_bytes()).unwrap();
        }
    }

    thread::sleep(Duration::from_millis(150));

    // Server collects all 50 messages
    let mut received_count = 0;
    while let Some(_pkt) = server.poll_incoming() {
        received_count += 1;
    }

    assert_eq!(received_count, 50, "Server must cleanly receive all 50 concurrent client messages");
}
