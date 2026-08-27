use nex_core::identity::recovery::shamir::{split_secret, combine_shares};

#[test]
fn test_r71_15_a_shamir_3_of_5_threshold_reconstruction() {
    let secret = [0x42u8; 32];
    let threshold = 3;
    let total_shares = 5;
    let epoch = 100;

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 7 % 250 + 1) as u8, ((i as u16 + 1) * 13 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&secret, threshold, total_shares, epoch, &random_coeffs).unwrap();
    assert_eq!(shares.len(), 5);

    // Reconstruct with any 3 shares (shares 0, 2, 4 -> indices 1, 3, 5)
    let subset = vec![shares[0].clone(), shares[2].clone(), shares[4].clone()];
    let reconstructed = combine_shares(&subset, threshold).expect("Reconstruction failed");
    assert_eq!(reconstructed, secret);
}

#[test]
fn test_r71_15_b_insufficient_shares_fails() {
    let secret = [0x55u8; 32];
    let threshold = 3;
    let total_shares = 5;

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 2) % 250 + 1) as u8, ((i as u16 + 5) % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&secret, threshold, total_shares, 1, &random_coeffs).unwrap();

    // Attempt reconstruction with only 2 shares < 3
    let subset = vec![shares[0].clone(), shares[1].clone()];
    let res = combine_shares(&subset, threshold);
    assert!(res.is_err(), "Fewer than threshold shares must fail");
}

#[test]
fn test_r71_15_c_all_combinations_reconstruct_exact_seed() {
    let secret = [0x77u8; 32];
    let threshold = 2;
    let total_shares = 4;

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 3) % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&secret, threshold, total_shares, 1, &random_coeffs).unwrap();

    // Test all pairs (0,1), (0,2), (0,3), (1,2), (1,3), (2,3)
    for i in 0..4 {
        for j in (i+1)..4 {
            let subset = vec![shares[i].clone(), shares[j].clone()];
            let recovered = combine_shares(&subset, threshold).unwrap();
            assert_eq!(recovered, secret, "Pair ({}, {}) failed", i, j);
        }
    }
}

#[test]
fn test_r71_15_d_duplicate_shares_rejected() {
    let secret = [0x88u8; 32];
    let threshold = 3;
    let total_shares = 5;

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) % 250 + 1) as u8, ((i as u16 + 2) % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&secret, threshold, total_shares, 1, &random_coeffs).unwrap();

    // Presenting share 0 twice plus share 1 (effective count is only 2 distinct shares)
    let subset = vec![shares[0].clone(), shares[0].clone(), shares[1].clone()];
    let res = combine_shares(&subset, threshold);
    assert!(res.is_err(), "Duplicate guardian share cannot satisfy threshold");
}

#[test]
fn test_r71_15_e_invalid_threshold_parameters() {
    let secret = [0x99u8; 32];
    let random_coeffs = vec![vec![1]; 32];

    // Threshold > total shares
    assert!(split_secret(&secret, 6, 5, 1, &random_coeffs).is_err());
    // Threshold == 0
    assert!(split_secret(&secret, 0, 5, 1, &random_coeffs).is_err());
}

#[test]
fn test_r71_15_f_tampered_share_corrupts_reconstruction() {
    let secret = [0xAAu8; 32];
    let threshold = 3;
    let total_shares = 5;

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 3 % 250 + 1) as u8, ((i as u16 + 1) * 7 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&secret, threshold, total_shares, 1, &random_coeffs).unwrap();

    let mut tampered_share = shares[0].clone();
    tampered_share.share_data[0] ^= 0xFF; // Flip bits in share 1

    let subset = vec![tampered_share, shares[1].clone(), shares[2].clone()];
    let recovered = combine_shares(&subset, threshold).unwrap();
    assert_ne!(recovered, secret, "Tampered share must not recover original secret");
}
