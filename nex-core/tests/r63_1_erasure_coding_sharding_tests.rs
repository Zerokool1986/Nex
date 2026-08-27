use nex_core::apps::resources::*;

#[test]
fn test_r63_1_a_split_into_k_plus_one_shards() {
    let payload = b"Hello, Sovereign Resource Network! Sharded across peers.".to_vec();
    let shards = ErasureCoder::split(&payload, 4);

    assert_eq!(shards.len(), 5); // 4 data + 1 parity
    assert_eq!(shards[0].shard_index, 0);
    assert_eq!(shards[4].shard_index, 4);
    assert_eq!(shards[0].total_shards, 5);
}

#[test]
fn test_r63_1_b_reconstruct_from_all_data_shards() {
    let payload = b"Cryptographic sharding and reconstruction test payload.".to_vec();
    let shards = ErasureCoder::split(&payload, 4);

    // Provide only the 4 data shards (indices 0, 1, 2, 3)
    let available: Vec<ShardDescriptor> = shards.into_iter().take(4).collect();
    let recovered = ErasureCoder::reconstruct(&available, 4, payload.len()).unwrap();

    assert_eq!(recovered, payload);
}

#[test]
fn test_r63_1_c_reconstruct_with_one_missing_shard_via_parity() {
    let payload = b"Resilient fault-tolerant peer-to-peer storage grid.".to_vec();
    let shards = ErasureCoder::split(&payload, 4);

    // Drop shard 1, keep shards 0, 2, 3, and parity (4)
    let mut available = Vec::new();
    available.push(shards[0].clone());
    available.push(shards[2].clone());
    available.push(shards[3].clone());
    available.push(shards[4].clone()); // parity

    let recovered = ErasureCoder::reconstruct(&available, 4, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn test_r63_1_d_reconstruct_fails_with_insufficient_shards() {
    let payload = b"Insufficient shards should fail gracefully.".to_vec();
    let shards = ErasureCoder::split(&payload, 4);

    // Drop 2 data shards, keep 2 data shards + parity (total 3 shards, need 4)
    let available = vec![shards[0].clone(), shards[1].clone(), shards[4].clone()];
    let res = ErasureCoder::reconstruct(&available, 4, payload.len());
    assert!(res.is_err());
}

#[test]
fn test_r63_1_e_arbitrary_length_preservation() {
    // 37 bytes (not a multiple of 4 or 8)
    let payload: Vec<u8> = (0..37).collect();
    let shards = ErasureCoder::split(&payload, 4);

    // Drop shard 0, use 1, 2, 3, 4 (parity)
    let available = vec![shards[1].clone(), shards[2].clone(), shards[3].clone(), shards[4].clone()];
    let recovered = ErasureCoder::reconstruct(&available, 4, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn test_r63_1_f_zero_regression_erasure_lifecycle() {
    let payload = vec![0xABu8; 1024];
    let shards = ErasureCoder::split(&payload, 8);
    let recovered = ErasureCoder::reconstruct(&shards[0..8], 8, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}
