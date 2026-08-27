use nex_core::storage::cdc::{FastCdcChunker, DEFAULT_MIN_CHUNK_SIZE, DEFAULT_AVG_CHUNK_SIZE, DEFAULT_MAX_CHUNK_SIZE};

#[test]
fn test_r69_1_a_empty_and_sub_min_size_chunking() {
    let chunker = FastCdcChunker::default();
    
    // Empty data
    let empty_chunks = chunker.chunk_slice(&[]);
    assert!(empty_chunks.is_empty());

    // Small data (< min size)
    let small_data = b"Hello sovereign world of NEX";
    let chunks = chunker.chunk_slice(small_data);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].offset, 0);
    assert_eq!(chunks[0].length, small_data.len());
    assert!(FastCdcChunker::verify_integrity(small_data, &chunks));
}

#[test]
fn test_r69_1_b_variable_content_defined_boundaries() {
    let chunker = FastCdcChunker::new(512, 2048, 8192);
    let mut data = Vec::with_capacity(64 * 1024);
    for i in 0..(64 * 1024) {
        data.push((i % 256) as u8);
    }

    let chunks = chunker.chunk_slice(&data);
    assert!(chunks.len() > 5, "Should generate multiple variable chunks");
    assert!(FastCdcChunker::verify_integrity(&data, &chunks));

    // Verify all chunk sizes are bounded between min and max (except possible tail)
    for (i, c) in chunks.iter().enumerate() {
        if i < chunks.len() - 1 {
            assert!(c.length >= 512, "Chunk length {} must be >= min 512", c.length);
            assert!(c.length <= 8192, "Chunk length {} must be <= max 8192", c.length);
        }
    }
}

#[test]
fn test_r69_1_c_deduplication_shift_invariance() {
    let chunker = FastCdcChunker::new(256, 1024, 4096);
    let mut base_data = Vec::with_capacity(128 * 1024);
    for i in 0..(128 * 1024) {
        base_data.push(((i * 7 + 13) % 256) as u8);
    }

    let base_chunks = chunker.chunk_slice(&base_data);

    // Insert 128 bytes prefix
    let mut shifted_data = vec![0xEEu8; 128];
    shifted_data.extend_from_slice(&base_data);

    let shifted_chunks = chunker.chunk_slice(&shifted_data);

    let base_hashes: std::collections::HashSet<[u8; 32]> = base_chunks.iter().map(|c| c.chunk_hash).collect();
    let mut shared_count = 0;
    for sc in &shifted_chunks {
        if base_hashes.contains(&sc.chunk_hash) {
            shared_count += 1;
        }
    }

    assert!(shared_count > 0, "FastCDC must preserve chunk boundaries despite prefix shifts (shared: {})", shared_count);
}

#[test]
fn test_r69_1_d_deterministic_boundary_hashing() {
    let chunker = FastCdcChunker::default();
    let data = vec![0xAAu8; 500 * 1024];

    let chunks1 = chunker.chunk_slice(&data);
    let chunks2 = chunker.chunk_slice(&data);

    assert_eq!(chunks1, chunks2, "FastCDC must be strictly deterministic across repeated runs");
}

#[test]
fn test_r69_1_e_large_payload_streaming_integrity() {
    let chunker = FastCdcChunker::new(DEFAULT_MIN_CHUNK_SIZE, DEFAULT_AVG_CHUNK_SIZE, DEFAULT_MAX_CHUNK_SIZE);
    let mut large_data = vec![0u8; 1024 * 1024]; // 1 MB
    for (i, b) in large_data.iter_mut().enumerate() {
        *b = ((i * 31 + 17) % 256) as u8;
    }

    let chunks = chunker.chunk_slice(&large_data);
    assert!(chunks.len() >= 4, "1MB payload should yield multiple chunks");
    assert!(FastCdcChunker::verify_integrity(&large_data, &chunks));
}

#[test]
fn test_r69_1_f_zero_corruption_detection() {
    let chunker = FastCdcChunker::new(512, 2048, 8192);
    let data = vec![0x55u8; 16 * 1024];
    let mut chunks = chunker.chunk_slice(&data);

    // Corrupt one chunk hash
    chunks[0].chunk_hash[0] ^= 0xFF;
    assert!(!FastCdcChunker::verify_integrity(&data, &chunks), "Integrity verification must reject corrupted hashes");
}
