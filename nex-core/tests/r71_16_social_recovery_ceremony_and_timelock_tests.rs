use nex_core::identity::master::NexMasterIdentity;
use nex_core::identity::recovery::shamir::split_secret;
use nex_core::identity::recovery::ceremony::SocialRecoveryCeremony;

#[test]
fn test_r71_16_a_social_recovery_ceremony_quorom_restores_root_seed() {
    let original_seed = [0x42u8; 32];
    let master = NexMasterIdentity::from_seed(&original_seed);

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 3 % 250 + 1) as u8, ((i as u16 + 1) * 5 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&original_seed, 3, 5, 1, &random_coeffs).unwrap();

    let mut ceremony = SocialRecoveryCeremony::new([0x01; 16], master.root_actor_id, 3, 1000);

    // Submit 3 shares
    ceremony.submit_share(shares[0].clone()).unwrap();
    ceremony.submit_share(shares[1].clone()).unwrap();
    ceremony.submit_share(shares[2].clone()).unwrap();

    // Time-lock expires at epoch 1000, finalize at epoch 1050
    let recovered_seed = ceremony.finalize_recovery(1050).expect("Recovery failed");
    assert_eq!(recovered_seed, original_seed);

    let recovered_master = NexMasterIdentity::from_seed(&recovered_seed);
    assert_eq!(recovered_master.root_actor_id, master.root_actor_id, "Root ActorID must be identical");
}

#[test]
fn test_r71_16_b_timelock_enforcement_prevents_premature_recovery() {
    let original_seed = [0x55u8; 32];
    let master = NexMasterIdentity::from_seed(&original_seed);

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 2 % 250 + 1) as u8, ((i as u16 + 1) * 4 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&original_seed, 3, 5, 1, &random_coeffs).unwrap();

    let mut ceremony = SocialRecoveryCeremony::new([0x02; 16], master.root_actor_id, 3, 1000); // Timelock = 1000

    ceremony.submit_share(shares[0].clone()).unwrap();
    ceremony.submit_share(shares[1].clone()).unwrap();
    ceremony.submit_share(shares[2].clone()).unwrap();

    // Attempt recovery at epoch 500 < 1000
    let res = ceremony.finalize_recovery(500);
    assert!(res.is_err(), "Premature recovery before time-lock expiry must be rejected");
}

#[test]
fn test_r71_16_c_full_quorum_bypasses_timelock() {
    let original_seed = [0x66u8; 32];
    let master = NexMasterIdentity::from_seed(&original_seed);

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 2 % 250 + 1) as u8, ((i as u16 + 1) * 4 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&original_seed, 3, 5, 1, &random_coeffs).unwrap();

    let mut ceremony = SocialRecoveryCeremony::new([0x03; 16], master.root_actor_id, 3, 1000);

    // If ALL 5 of 5 guardians participate, recovery succeeds immediately without waiting for timelock
    for s in &shares {
        ceremony.submit_share(s.clone()).unwrap();
    }

    let recovered_seed = ceremony.finalize_recovery(500).expect("Full quorum must succeed immediately");
    assert_eq!(recovered_seed, original_seed);
}

#[test]
fn test_r71_16_d_owner_cancels_social_recovery() {
    let original_seed = [0x77u8; 32];
    let master = NexMasterIdentity::from_seed(&original_seed);

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 2 % 250 + 1) as u8, ((i as u16 + 1) * 4 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&original_seed, 3, 5, 1, &random_coeffs).unwrap();

    let mut ceremony = SocialRecoveryCeremony::new([0x04; 16], master.root_actor_id, 3, 1000);
    ceremony.submit_share(shares[0].clone()).unwrap();
    ceremony.submit_share(shares[1].clone()).unwrap();
    ceremony.submit_share(shares[2].clone()).unwrap();

    // Owner detects unauthorized recovery and cancels it
    ceremony.cancel_ceremony();

    let res = ceremony.finalize_recovery(1050);
    assert!(res.is_err(), "Canceled recovery ceremony must not finalize");
}

#[test]
fn test_r71_16_e_duplicate_share_submission_rejected() {
    let original_seed = [0x88u8; 32];
    let master = NexMasterIdentity::from_seed(&original_seed);

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 2 % 250 + 1) as u8, ((i as u16 + 1) * 4 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&original_seed, 3, 5, 1, &random_coeffs).unwrap();

    let mut ceremony = SocialRecoveryCeremony::new([0x05; 16], master.root_actor_id, 3, 1000);
    ceremony.submit_share(shares[0].clone()).unwrap();
    assert!(ceremony.submit_share(shares[0].clone()).is_err(), "Duplicate share must be rejected");
}

#[test]
fn test_r71_16_f_insufficient_shares_cannot_finalize() {
    let original_seed = [0x99u8; 32];
    let master = NexMasterIdentity::from_seed(&original_seed);

    let random_coeffs: Vec<Vec<u8>> = (0..32)
        .map(|i| vec![((i as u16 + 1) * 2 % 250 + 1) as u8, ((i as u16 + 1) * 4 % 250 + 1) as u8])
        .collect();

    let shares = split_secret(&original_seed, 3, 5, 1, &random_coeffs).unwrap();

    let mut ceremony = SocialRecoveryCeremony::new([0x06; 16], master.root_actor_id, 3, 1000);
    ceremony.submit_share(shares[0].clone()).unwrap();
    ceremony.submit_share(shares[1].clone()).unwrap();
    // Only 2 shares submitted < threshold 3

    let res = ceremony.finalize_recovery(1050);
    assert!(res.is_err(), "Finalizing with insufficient shares must fail");
}
