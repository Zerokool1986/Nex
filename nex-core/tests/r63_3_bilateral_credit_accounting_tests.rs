use nex_core::apps::resources::*;

#[test]
fn test_r63_3_a_initial_credit_balance() {
    let ledger = BilateralCreditLedger::new();
    let peer = [0x01u8; 32];
    assert_eq!(ledger.get_balance(&peer), 0);
}

#[test]
fn test_r63_3_b_positive_resource_provision() {
    let mut ledger = BilateralCreditLedger::new();
    let peer = [0x01u8; 32];

    // We provide 10MB to peer
    let new_bal = ledger.record_transfer(peer, 10 * 1024 * 1024, 100 * 1024 * 1024).unwrap();
    assert_eq!(new_bal, 10 * 1024 * 1024);
    assert_eq!(ledger.get_balance(&peer), 10 * 1024 * 1024);
}

#[test]
fn test_r63_3_c_resource_consumption_debit() {
    let mut ledger = BilateralCreditLedger::new();
    let peer = [0x01u8; 32];

    // We consume 4MB from peer
    let new_bal = ledger.record_transfer(peer, -4 * 1024 * 1024, 100 * 1024 * 1024).unwrap();
    assert_eq!(new_bal, -4 * 1024 * 1024);
    assert_eq!(ledger.get_balance(&peer), -4 * 1024 * 1024);
}

#[test]
fn test_r63_3_d_debt_ceiling_enforcement() {
    let mut ledger = BilateralCreditLedger::new();
    let peer = [0x02u8; 32];
    let debt_ceiling = 10 * 1024 * 1024; // 10MB max debt

    // Try to consume 15MB from peer (exceeds ceiling)
    let res = ledger.record_transfer(peer, -15 * 1024 * 1024, debt_ceiling);
    assert!(res.is_err());
    assert_eq!(ledger.get_balance(&peer), 0, "Balance must remain unchanged after rejection");
}

#[test]
fn test_r63_3_e_multi_peer_isolation() {
    let mut ledger = BilateralCreditLedger::new();
    let p1 = [0x01u8; 32];
    let p2 = [0x02u8; 32];

    ledger.record_transfer(p1, 500, 1000).unwrap();
    ledger.record_transfer(p2, -300, 1000).unwrap();

    assert_eq!(ledger.get_balance(&p1), 500);
    assert_eq!(ledger.get_balance(&p2), -300);
}

#[test]
fn test_r63_3_f_zero_regression_credit_lifecycle() {
    let mut ledger = BilateralCreditLedger::new();
    let peer = [0x05u8; 32];
    for _ in 0..10 {
        ledger.record_transfer(peer, 100, 10000).unwrap();
    }
    assert_eq!(ledger.get_balance(&peer), 1000);
}
