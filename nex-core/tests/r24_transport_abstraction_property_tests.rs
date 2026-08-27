use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use nex_core::transport::types::{
    encode_frame, decode_frame, TransportError
};
use nex_core::transport::fragmentation::{
    fragment_payload, FragmentationReassembler
};
use nex_core::transport::adapter::{
    MockReticulumAdapter, MockQuicAdapter
};
use nex_core::transport::dispatcher::MultiTransportDispatcher;
use nex_core::sync::types::{SyncMessage, StatusAnnouncement};
use nex_core::model::Boundary;

#[test]
fn test_r24_b_framing_and_crc32_bit_flip_detection() {
    let payload = b"Hello Nex Decentralized Network!".to_vec();
    let frame = encode_frame(0x02, 0x00, &payload);

    // 1. Valid frame decoding
    let (tag, flags, decoded_payload) = decode_frame(&frame).expect("Valid frame must decode");
    assert_eq!(tag, 0x02);
    assert_eq!(flags, 0x00);
    assert_eq!(decoded_payload, payload);

    // 2. Corrupt 1 byte in payload -> CRC32 failure
    let mut corrupted_frame = frame.clone();
    let payload_byte_idx = 13 + 5; // Header is 13 bytes
    corrupted_frame[payload_byte_idx] ^= 0xFF;

    let res_corrupted = decode_frame(&corrupted_frame);
    assert!(matches!(res_corrupted, Err(TransportError::CorruptedFrame(_))), "R24-B: Bit flip in payload must trigger CRC32 failure");
}

#[test]
fn test_r24_c_low_mtu_fragmentation_and_reassembly_128kb() {
    let mut rng = ChaCha20Rng::seed_from_u64(0x4242_1337);
    let mut large_payload = vec![0u8; 128 * 1024]; // 128 KB simulated ZK proof artifact
    for byte in large_payload.iter_mut() {
        *byte = (rand::Rng::gen::<u8>(&mut rng)) ;
    }

    let msg_id = [0x55; 32];
    let reticulum_mtu = 500; // Low-MTU mesh

    // Fragment 128KB payload into 500-byte MTU chunks
    let chunks = fragment_payload(msg_id, &large_payload, reticulum_mtu).expect("Fragmentation must succeed");
    assert!(chunks.len() > 270, "128KB / ~464B chunk capacity should yield ~283 chunks");

    // Shuffle chunks to simulate out-of-order packet delivery over mesh
    let mut shuffled_chunks = chunks.clone();
    shuffled_chunks.shuffle(&mut rng);

    let mut reassembler = FragmentationReassembler::new();
    let mut final_payload = None;

    for chunk in shuffled_chunks {
        if let Some(assembled) = reassembler.ingest_chunk(&chunk).expect("Chunk ingestion must succeed") {
            final_payload = Some(assembled);
        }
    }

    let reassembled_bytes = final_payload.expect("All chunks received, payload must reassemble");
    assert_eq!(reassembled_bytes, large_payload, "R24-C: Reassembled 128KB payload must match original byte-for-byte");
}

#[test]
fn test_r24_d_and_e_multi_transport_dispatch_and_failover() {
    let mut dispatcher = MultiTransportDispatcher::new();
    dispatcher.register_adapter(Box::new(MockReticulumAdapter::new()));
    dispatcher.register_adapter(Box::new(MockQuicAdapter::new()));

    let dest = b"peer_address";
    let payload = b"payload_bytes";

    // 1. Dispatch directly over QUIC (tag 0x02)
    let used_tag_quic = dispatcher.dispatch(0x02, dest, payload, false).expect("QUIC dispatch must succeed");
    assert_eq!(used_tag_quic, 0x02);

    // 2. Simulate QUIC interface going down
    dispatcher.adapters.remove(&0x02);
    let mut dead_quic = MockQuicAdapter::new();
    dead_quic.connected = false;
    dispatcher.register_adapter(Box::new(dead_quic));

    // 3. Dispatch to QUIC with allow_failover: true -> Automatically fails over to Reticulum (tag 0x01)
    let failover_tag = dispatcher.dispatch(0x02, dest, payload, true).expect("Failover to Reticulum must succeed");
    assert_eq!(failover_tag, 0x01, "R24-E: Must failover to Reticulum when QUIC is offline");

    // 4. Dispatch with allow_failover: false -> Returns NoRoutableTransport
    let res_no_failover = dispatcher.dispatch(0x02, dest, payload, false);
    assert_eq!(res_no_failover, Err(TransportError::NoRoutableTransport));
}

#[test]
fn test_r24_g_end_to_end_sync_roundtrip_over_transport() {
    let announcement = SyncMessage::StatusAnnouncement(StatusAnnouncement {
        node_id: "NodeA".into(),
        latest_checkpoint_id: [0xAA; 32],
        frontier: vec![[0x11; 32], [0x22; 32]],
        boundary: Boundary { max_epoch: 5, max_lamport: 20 },
    });

    // 1. Serialize SyncMessage
    let serialized_sync = serde_json::to_vec(&announcement).expect("Serialization must succeed");

    // 2. Encode into physical transport frame
    let framed_packet = encode_frame(0x01, 0x00, &serialized_sync);

    // 3. Low-MTU mesh chunking
    let msg_id = [0x77; 32];
    let chunks = fragment_payload(msg_id, &framed_packet, 200).expect("Chunking must succeed");

    // 4. Receive and reassemble on peer node
    let mut peer_reassembler = FragmentationReassembler::new();
    let mut peer_framed_bytes = None;
    for chunk in chunks {
        if let Some(reassembled) = peer_reassembler.ingest_chunk(&chunk).unwrap() {
            peer_framed_bytes = Some(reassembled);
        }
    }

    let received_frame = peer_framed_bytes.expect("Frame must reassemble");

    // 5. Decode transport frame
    let (tag, _, payload_bytes) = decode_frame(&received_frame).expect("Decoding must succeed");
    assert_eq!(tag, 0x01);

    // 6. Deserialize back to SyncMessage
    let peer_sync_msg: SyncMessage = serde_json::from_slice(&payload_bytes).expect("SyncMessage must deserialize");
    assert_eq!(peer_sync_msg, announcement, "R24-G: End-to-end sync message roundtrip over transport must match exactly");
}

#[test]
fn test_r24_h_transport_error_isolation() {
    // 1. Short truncated header
    let res_short = decode_frame(&[0x4E, 0x58, 0x01]);
    assert!(matches!(res_short, Err(TransportError::CorruptedFrame(_))));

    // 2. Invalid magic bytes
    let garbage_frame = vec![0xFF; 20];
    let res_magic = decode_frame(&garbage_frame);
    assert!(matches!(res_magic, Err(TransportError::CorruptedFrame(_))));
}

trait AsAny {
    fn as_any_mut(&mut self) -> Option<&mut MockQuicAdapter> { None }
}
