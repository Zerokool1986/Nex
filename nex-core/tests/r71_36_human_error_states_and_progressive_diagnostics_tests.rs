use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::reality::{ProductionRealityEngine, NetworkLinkState};

#[test]
fn test_r71_36_a_offline_human_viewmodel_reassurance() {
    let vm = ProductionRealityEngine::format_network_status(
        NetworkLinkState::Offline,
        3,
        InterfaceComplexity::Simple,
    );

    assert_eq!(vm.headline, "You're offline");
    assert!(vm.detail_message.contains("Changes will sync automatically"));
    assert_eq!(vm.pending_items_count, 3);
    assert!(vm.technical_error_code.is_none());
}

#[test]
fn test_r71_36_b_expert_tier_exposes_physical_error_codes() {
    let vm_exp = ProductionRealityEngine::format_network_status(
        NetworkLinkState::Offline,
        3,
        InterfaceComplexity::Expert,
    );

    assert_eq!(vm_exp.headline, "You're offline");
    assert!(vm_exp.technical_error_code.is_some());
    assert!(vm_exp.technical_error_code.unwrap().contains("ERR_NO_ROUTE"));
}

#[test]
fn test_r71_36_c_reconnecting_human_state() {
    let vm = ProductionRealityEngine::format_network_status(
        NetworkLinkState::Reconnecting,
        2,
        InterfaceComplexity::Standard,
    );

    assert_eq!(vm.headline, "Reconnecting…");
    assert!(vm.can_retry_now);
}

#[test]
fn test_r71_36_d_partial_connectivity_state() {
    let vm = ProductionRealityEngine::format_network_status(
        NetworkLinkState::PartialConnectivity,
        5,
        InterfaceComplexity::Simple,
    );

    assert_eq!(vm.headline, "Some items are waiting");
    assert_eq!(vm.pending_items_count, 5);
}

#[test]
fn test_r71_36_e_lossy_degraded_sync_state() {
    let vm = ProductionRealityEngine::format_network_status(
        NetworkLinkState::DegradedLossy(40),
        1,
        InterfaceComplexity::Standard,
    );

    assert_eq!(vm.headline, "Syncing slowly…");
    assert!(vm.detail_message.contains("retransmitting packets"));
}

#[test]
fn test_r71_36_f_local_wifi_direct_up_to_date_state() {
    let vm = ProductionRealityEngine::format_network_status(
        NetworkLinkState::ConnectedLocalWifi,
        0,
        InterfaceComplexity::Simple,
    );

    assert_eq!(vm.headline, "Up to date");
    assert_eq!(vm.detail_message, "Protected on your local mesh");
}
