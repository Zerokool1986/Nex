use nex_core::apps::erasure::ReedSolomonEngine;

#[test]
fn test_r69_3_a_exact_data_reconstruction_from_all_data_shards() {
    let rs = ReedSolomonEngine::new();
    let original = b"Sovereign distributed computing across family and social mesh nodes.";
    
    // (K=4, M=2)
    let shards = rs.encode(original, 4, 2);
    assert_eq!(shards.len(), 6);

    // Provide only original 4 data shards (0..4)
    let available = &shards[0..4];
    let recovered = rs.decode(available, 4, 2, original.len()).expect("Decode must succeed");

    assert_eq!(recovered, original);
}

#[test]
fn test_r69_3_b_reconstruction_with_lost_data_shards_using_parity() {
    let rs = ReedSolomonEngine::new();
    let original = b"Cryptographic capabilities govern all namespace access in NEX.";
    
    // (K=4, M=2): tolerate up to 2 lost shards
    let shards = rs.encode(original, 4, 2);

    // Drop data shards 0 and 2; keep shards 1, 3, 4 (parity 0), 5 (parity 1)
    let available = vec![shards[1].clone(), shards[3].clone(), shards[4].clone(), shards[5].clone()];
    assert_eq!(available.len(), 4);

    let recovered = rs.decode(&available, 4, 2, original.len()).expect("Reconstruction from parity must succeed");
    assert_eq!(recovered, original);
}

#[test]
fn test_r69_3_c_high_order_k8_m4_reconstruction() {
    let rs = ReedSolomonEngine::new();
    let mut large_payload = vec![0u8; 64 * 1024]; // 64 KB
    for (i, b) in large_payload.iter_mut().enumerate() {
        *b = ((i * 13 + 37) % 256) as u8;
    }

    // (K=8, M=4): Total 12 shards, tolerate 4 failures
    let shards = rs.encode(&large_payload, 8, 4);
    assert_eq!(shards.len(), 12);

    // Select any 8 shards (e.g. indices 0, 2, 4, 6, 8, 9, 10, 11)
    let available = vec![
        shards[0].clone(),
        shards[2].clone(),
        shards[4].clone(),
        shards[6].clone(),
        shards[8].clone(),
        shards[9].clone(),
        shards[10].clone(),
        shards[11].clone(),
    ];

    let recovered = rs.decode(&available, 8, 4, large_payload.len()).expect("High-order decode must succeed");
    assert_eq!(recovered, large_payload);
}

#[test]
fn test_r69_3_d_insufficient_shards_detection() {
    let rs = ReedSolomonEngine::new();
    let original = b"Short payload for fault test";
    
    // (K=4, M=2)
    let shards = rs.encode(original, 4, 2);

    // Only provide 3 shards (need 4)
    let available = &shards[0..3];
    let result = rs.decode(available, 4, 2, original.len());

    assert!(result.is_err(), "Must fail when fewer than K shards are provided");
}

#[test]
fn test_r69_3_e_arbitrary_payload_lengths_padding_integrity() {
    let rs = ReedSolomonEngine::new();
    // Prime length payload that does not divide evenly into K=5
    let original = vec![0x42u8; 1009];

    let shards = rs.encode(&original, 5, 3);
    assert_eq!(shards.len(), 8);

    // Use shards 1, 2, 4, 5, 7
    let available = vec![
        shards[1].clone(),
        shards[2].clone(),
        shards[4].clone(),
        shards[5].clone(),
        shards[7].clone(),
    ];

    let recovered = rs.decode(&available, 5, 3, original.len()).expect("Uneven length decode must succeed");
    assert_eq!(recovered.len(), 1009);
    assert_eq!(recovered, original);
}

#[test]
fn test_r69_3_f_deterministic_galois_field_arithmetic() {
    let rs = ReedSolomonEngine::new();
    let payload = vec![0xFFu8; 100];

    let shards1 = rs.encode(&payload, 3, 2);
    let shards2 = rs.encode(&payload, 3, 2);

    assert_eq!(shards1, shards2, "Reed-Solomon encoding must be strictly deterministic");
}
