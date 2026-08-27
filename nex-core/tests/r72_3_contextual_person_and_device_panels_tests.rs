use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::product::person::{PersonPanelController, TrustTier};
use nex_core::product::device::DevicePanelController;
use nex_core::product::settings::SettingsController;
use nex_core::identity::types::{DeviceCertificate, KeyType};
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r72_3_a_person_panel_exposes_verified_trust_and_actions() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x01u8; 32]));
    node.start().unwrap();

    let amy_actor = [0xAA; 32];
    let person = PersonPanelController::build_person_surface(
        &node,
        &amy_actor,
        "Amy",
        TrustTier::VerifiedSovereignPeer,
        InterfaceComplexity::Standard,
    );

    assert_eq!(person.display_name, "Amy");
    assert!(person.trust_badge.contains("Verified"));
    assert_eq!(person.quick_actions.len(), 4);
    assert!(person.quick_actions.contains(&"Send Message".to_string()));
    assert!(person.quick_actions.contains(&"Share Photo".to_string()));
}

#[test]
fn test_r72_3_b_device_panel_exposes_local_host_and_hardware_keystore() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x02u8; 32]);
    let pk = key.verifying_key().to_bytes();
    let actor_id = derive_actor_id(KeyType::Ed25519, &pk);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let dev = DevicePanelController::build_device_surface(
        &node,
        &actor_id,
        "Chris's Pixel 9 Pro XL",
        None,
        false,
        true, // Hardware verified on mobile host
        InterfaceComplexity::Standard,
    );

    assert_eq!(dev.device_name, "Chris's Pixel 9 Pro XL");
    assert!(dev.is_local_device);
    assert!(dev.hardware_keystore_backed);
    assert_eq!(dev.latency_ms, 1);
}

#[test]
fn test_r72_3_c_device_panel_revocation_badge() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x03u8; 32]);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let remote_actor = [0x55; 32];
    let cert = DeviceCertificate {
        master_actor_id: [0x11; 32],
        device_actor_id: remote_actor,
        not_before_epoch: 10,
        expires_at_epoch: 50,
        master_pubkey: None,
        signature: vec![0; 64],
    };

    let dev = DevicePanelController::build_device_surface(
        &node,
        &remote_actor,
        "Stolen Laptop",
        Some(&cert),
        true, // Revoked
        false,
        InterfaceComplexity::Standard,
    );

    assert!(!dev.is_local_device);
    assert!(dev.connection_badge.contains("Revoked"));
}

#[test]
fn test_r72_3_d_settings_consequence_tree_consequence_layers() {
    let settings = SettingsController::build_settings_tree(InterfaceComplexity::Standard);
    assert_eq!(settings.user_section.len(), 2);
    assert_eq!(settings.your_nex_section.len(), 3);
    assert_eq!(settings.applications_section.len(), 2);
    assert_eq!(settings.system_section.len(), 1);
    assert!(settings.advanced_section.is_none());
}

#[test]
fn test_r72_3_e_settings_advanced_tier_exposes_smt_and_wal_inspectors() {
    let settings_adv = SettingsController::build_settings_tree(InterfaceComplexity::Advanced);
    assert!(settings_adv.advanced_section.is_some());
    let adv = settings_adv.advanced_section.unwrap();
    assert_eq!(adv.len(), 2);
    assert_eq!(adv[0].key, "smt");
    assert_eq!(adv[1].key, "wal");
}

#[test]
fn test_r72_3_f_experience_slider_independent_of_authorization() {
    let s_simple = SettingsController::build_settings_tree(InterfaceComplexity::Simple);
    let s_expert = SettingsController::build_settings_tree(InterfaceComplexity::Expert);

    assert_eq!(s_simple.user_section[0].current_value, "Chris");
    assert_eq!(s_expert.user_section[0].current_value, "Chris");
    assert!(s_expert.advanced_section.is_some());
}
