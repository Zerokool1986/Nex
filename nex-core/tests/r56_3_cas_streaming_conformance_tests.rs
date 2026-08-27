use sha2::{Sha256, Digest};
use nex_core::apps::drive::CasChunkStore;
use nex_core::ffi::stream::*;
use nex_core::ffi::handle::*;

#[test]
fn test_r56_3_a_48byte_header_serialization_and_parsing() {
    let mut digest = [0u8; 32];
    digest[0] = 0xAA;
    digest[31] = 0xBB;

    let header = CasStreamHeader {
        magic: CAS_STREAM_MAGIC,
        opcode: OP_PUT_CHUNK,
        flags: 0x0001,
        payload_len: 1024,
        reserved: 0,
        expected_digest: digest,
    };

    let serialized = header.serialize();
    assert_eq!(serialized.len(), 48, "Total header frame size must be exactly 48 bytes");

    let parsed = CasStreamHeader::parse(&serialized).unwrap();
    assert_eq!(parsed, header);
}

#[test]
fn test_r56_3_b_cas_chunk_streaming_and_sha256_verification() {
    let mut cas = CasChunkStore::new();
    let payload = b"High throughput 4K video frame buffer chunk data bytes";
    let actual_digest: [u8; 32] = Sha256::digest(payload).into();

    let header = CasStreamHeader {
        magic: CAS_STREAM_MAGIC,
        opcode: OP_PUT_CHUNK,
        flags: 0,
        payload_len: payload.len() as u32,
        reserved: 0,
        expected_digest: actual_digest,
    };

    let stored_digest = CasStreamProcessor::process_put_chunk(&mut cas, &header, payload).unwrap();
    assert_eq!(stored_digest, actual_digest);

    let retrieved = CasStreamProcessor::process_get_chunk(&cas, &actual_digest).unwrap();
    assert_eq!(retrieved, payload);
}

#[test]
fn test_r56_3_c_corrupted_chunk_rejection() {
    let mut cas = CasChunkStore::new();
    let payload = b"Genuine uncorrupted chunk content";
    let mut wrong_digest = [0u8; 32];
    wrong_digest[0] = 0xFF; // Tampered digest

    let header = CasStreamHeader {
        magic: CAS_STREAM_MAGIC,
        opcode: OP_PUT_CHUNK,
        flags: 0,
        payload_len: payload.len() as u32,
        reserved: 0,
        expected_digest: wrong_digest,
    };

    let result = CasStreamProcessor::process_put_chunk(&mut cas, &header, payload);
    assert_eq!(result, Err(NEX_ERR_CAS_CORRUPTION));
}

#[test]
fn test_r56_3_d_deduplicated_chunk_ingestion_idempotency() {
    let mut cas = CasChunkStore::new();
    let payload = b"Deduplicated master raw photo chunk";
    let digest: [u8; 32] = Sha256::digest(payload).into();

    let header = CasStreamHeader {
        magic: CAS_STREAM_MAGIC,
        opcode: OP_PUT_CHUNK,
        flags: 0,
        payload_len: payload.len() as u32,
        reserved: 0,
        expected_digest: digest,
    };

    // First put
    assert!(CasStreamProcessor::process_put_chunk(&mut cas, &header, payload).is_ok());
    assert_eq!(cas.chunks.len(), 1);

    // Second duplicate put
    assert!(CasStreamProcessor::process_put_chunk(&mut cas, &header, payload).is_ok());
    assert_eq!(cas.chunks.len(), 1, "Duplicate chunk must not increase store count");
}

#[test]
fn test_r56_3_e_large_payload_chunk_streaming() {
    let mut cas = CasChunkStore::new();
    let payload = vec![0x42u8; 1024 * 1024]; // 1 MB chunk
    let digest: [u8; 32] = Sha256::digest(&payload).into();

    let header = CasStreamHeader {
        magic: CAS_STREAM_MAGIC,
        opcode: OP_PUT_CHUNK,
        flags: 0,
        payload_len: payload.len() as u32,
        reserved: 0,
        expected_digest: digest,
    };

    assert!(CasStreamProcessor::process_put_chunk(&mut cas, &header, &payload).is_ok());
    let retrieved = CasStreamProcessor::process_get_chunk(&cas, &digest).unwrap();
    assert_eq!(retrieved.len(), 1024 * 1024);
}

#[test]
fn test_r56_3_f_zero_regression_across_cas_stream_lifecycle() {
    let mut cas = CasChunkStore::new();

    for i in 0..20 {
        let payload = format!("chunk_content_iteration_{}", i).into_bytes();
        let digest: [u8; 32] = Sha256::digest(&payload).into();

        let header = CasStreamHeader {
            magic: CAS_STREAM_MAGIC,
            opcode: OP_PUT_CHUNK,
            flags: 0,
            payload_len: payload.len() as u32,
            reserved: 0,
            expected_digest: digest,
        };

        assert!(CasStreamProcessor::process_put_chunk(&mut cas, &header, &payload).is_ok());
        let read_back = CasStreamProcessor::process_get_chunk(&cas, &digest).unwrap();
        assert_eq!(read_back, payload);
    }
    assert_eq!(cas.chunks.len(), 20);
}
