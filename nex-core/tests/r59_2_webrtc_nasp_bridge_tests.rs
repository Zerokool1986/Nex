use nex_core::apps::web::WebRtcNaspBridge;

#[test]
fn test_r59_2_a_webrtc_framing_and_unframing() {
    let payload = b"Ephemeral NASP Session Handshake Frame Over WebRTC";
    let framed = WebRtcNaspBridge::frame_data_channel_message(payload);

    assert_eq!(framed.len(), 4 + payload.len());
    let unframed = WebRtcNaspBridge::unframe_data_channel_message(&framed).unwrap();
    assert_eq!(unframed, payload);
}

#[test]
fn test_r59_2_b_webrtc_incomplete_frame_rejection() {
    let too_short = vec![0x00, 0x00];
    assert!(WebRtcNaspBridge::unframe_data_channel_message(&too_short).is_err());

    let payload_truncated = vec![0x00, 0x00, 0x00, 0x10, 0xAA, 0xBB];
    assert!(WebRtcNaspBridge::unframe_data_channel_message(&payload_truncated).is_err());
}

#[test]
fn test_r59_2_c_webrtc_bidirectional_exchange() {
    let client_msg = b"Client->Node Request";
    let client_frame = WebRtcNaspBridge::frame_data_channel_message(client_msg);

    let node_received = WebRtcNaspBridge::unframe_data_channel_message(&client_frame).unwrap();
    assert_eq!(node_received, client_msg);

    let node_resp = b"Node->Client Response";
    let node_frame = WebRtcNaspBridge::frame_data_channel_message(node_resp);

    let client_received = WebRtcNaspBridge::unframe_data_channel_message(&node_frame).unwrap();
    assert_eq!(client_received, node_resp);
}

#[test]
fn test_r59_2_d_webrtc_multi_frame_stream() {
    let mut stream = Vec::new();
    for i in 0..5 {
        let msg = format!("Frame {}", i);
        stream.extend_from_slice(&WebRtcNaspBridge::frame_data_channel_message(msg.as_bytes()));
    }

    let mut cursor = 0;
    let mut count = 0;
    while cursor < stream.len() {
        let slice = &stream[cursor..];
        let unframed = WebRtcNaspBridge::unframe_data_channel_message(slice).unwrap();
        let expected = format!("Frame {}", count);
        assert_eq!(unframed, expected.as_bytes());
        cursor += 4 + unframed.len();
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn test_r59_2_e_webrtc_empty_payload() {
    let empty = b"";
    let framed = WebRtcNaspBridge::frame_data_channel_message(empty);
    assert_eq!(framed, vec![0x00, 0x00, 0x00, 0x00]);
    let unframed = WebRtcNaspBridge::unframe_data_channel_message(&framed).unwrap();
    assert_eq!(unframed, empty);
}

#[test]
fn test_r59_2_f_zero_regression_webrtc_lifecycle() {
    for _ in 0..10 {
        let payload = vec![0xFFu8; 1024];
        let framed = WebRtcNaspBridge::frame_data_channel_message(&payload);
        let unframed = WebRtcNaspBridge::unframe_data_channel_message(&framed).unwrap();
        assert_eq!(unframed, &payload[..]);
    }
}
