use nex_core::apps::discovery::*;

#[test]
fn test_r62_2_a_direct_petname_resolution() {
    let mut wot = WebOfTrustRegistry::new();
    let alice = [0x01u8; 32];
    let bob = [0x02u8; 32];

    wot.add_alias(alice, bob, "Bob", 0.9);

    let resolved = wot.resolve_alias(&alice, "bob");
    assert!(resolved.is_some());
    let (target, score) = resolved.unwrap();
    assert_eq!(target, bob);
    assert_eq!(score, 0.9);
}

#[test]
fn test_r62_2_b_transitive_2hop_resolution_attenuation() {
    let mut wot = WebOfTrustRegistry::new();
    let alice = [0x01u8; 32];
    let bob = [0x02u8; 32];
    let charlie = [0x03u8; 32];

    wot.add_alias(alice, bob, "Bob", 0.8);
    wot.add_alias(bob, charlie, "Charlie", 0.8);

    let resolved = wot.resolve_alias(&alice, "charlie");
    assert!(resolved.is_some());
    let (target, score) = resolved.unwrap();
    assert_eq!(target, charlie);
    // Score should be 0.8 * 0.8 * 0.5 = 0.32
    assert!((score - 0.32).abs() < 1e-6, "Transitive score should be attenuated to ~0.32, was {}", score);
}

#[test]
fn test_r62_2_c_case_insensitive_alias_lookup() {
    let mut wot = WebOfTrustRegistry::new();
    let alice = [0x01u8; 32];
    let bob = [0x02u8; 32];

    wot.add_alias(alice, bob, "WorkLaptop", 1.0);

    assert!(wot.resolve_alias(&alice, "worklaptop").is_some());
    assert!(wot.resolve_alias(&alice, "WORKLAPTOP").is_some());
    assert!(wot.resolve_alias(&alice, "WorkLaptop").is_some());
}

#[test]
fn test_r62_2_d_unknown_alias_returns_none() {
    let mut wot = WebOfTrustRegistry::new();
    let alice = [0x01u8; 32];

    assert!(wot.resolve_alias(&alice, "stranger").is_none());
}

#[test]
fn test_r62_2_e_confidence_score_clamping() {
    let mut wot = WebOfTrustRegistry::new();
    let alice = [0x01u8; 32];
    let bob = [0x02u8; 32];

    wot.add_alias(alice, bob, "Bob", 1.5);
    let (_, score) = wot.resolve_alias(&alice, "bob").unwrap();
    assert_eq!(score, 1.0, "Confidence score must be clamped at 1.0");
}

#[test]
fn test_r62_2_f_zero_regression_wot_lifecycle() {
    let mut wot = WebOfTrustRegistry::new();
    for i in 0..10 {
        let a = [i; 32];
        let b = [i + 1; 32];
        wot.add_alias(a, b, "Friend", 0.5);
        assert!(wot.resolve_alias(&a, "friend").is_some());
    }
}
