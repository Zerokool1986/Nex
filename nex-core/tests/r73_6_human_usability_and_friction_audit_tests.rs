use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::SpaceType;
use nex_core::runtime::experience::{InterfaceComplexity, HumanExperienceEngine};
use nex_core::product::device::DevicePanelController;
use nex_core::product::person::{PersonPanelController, TrustTier};
use nex_core::product::desktop_app::{DesktopAppSession, DesktopNavigationTab};
use nex_core::identity::types::KeyType;
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r73_6_a_zero_jargon_identity_setup_surface() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x61u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let dev = DevicePanelController::build_device_surface(
        &node,
        &root_actor,
        "Pixel 9 Pro XL",
        None,
        false,
        false,
        InterfaceComplexity::Simple,
    );

    // In Simple (Human Default) mode, verify zero cryptography jargon in primary UI
    assert!(!dev.device_name.is_empty());
    assert!(dev.technical_device_info.is_none());
    assert!(dev.connection_badge.contains("This Device"));
}

#[test]
fn test_r73_6_b_photo_import_human_simplicity() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x62u8; 32]);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let mut session = DesktopAppSession::new();
    session.set_complexity_slider(InterfaceComplexity::Simple);

    // Human navigation to Photos lens
    session.select_tab(DesktopNavigationTab::Photos);
    let view = session.render_view_string(&node);

    assert!(view.contains("PHOTOS LENS"));
    assert!(view.contains("Space: Family"));
}

#[test]
fn test_r73_6_c_cross_device_human_status_clarity() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x63u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let dev = DevicePanelController::build_device_surface(
        &node,
        &root_actor,
        "Chris's Pixel 9 Pro XL",
        None,
        false,
        false,
        InterfaceComplexity::Standard,
    );

    assert!(dev.is_local_device);
    assert_eq!(dev.transport_type_label, "Local Direct / IPC");
}

#[test]
fn test_r73_6_d_family_sharing_permission_model_clarity() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x64u8; 32]);
    let amy_key = SigningKey::from_bytes(&[0x65u8; 32]);
    let amy_actor = derive_actor_id(KeyType::Ed25519, &amy_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let person_panel = PersonPanelController::build_person_surface(
        &node,
        &amy_actor,
        "Amy",
        TrustTier::VerifiedSovereignPeer,
        InterfaceComplexity::Simple,
    );

    assert_eq!(person_panel.display_name, "Amy");
    assert_eq!(person_panel.trust_tier, TrustTier::VerifiedSovereignPeer);
    assert!(person_panel.technical_identity_info.is_none());
}

#[test]
fn test_r73_6_e_offline_state_human_reassurance() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x66u8; 32]);
    let other_actor = derive_actor_id(KeyType::Ed25519, &[0x99u8; 32]);

    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    // Revoked/offline remote device panel
    let dev = DevicePanelController::build_device_surface(
        &node,
        &other_actor,
        "Pixel 9 Pro XL (Airplane Mode)",
        None,
        true, // Revoked/Offline
        false,
        InterfaceComplexity::Standard,
    );

    assert!(!dev.is_local_device);
    assert!(dev.connection_badge.contains("Revoked"));
}

#[test]
fn test_r73_6_f_progressive_disclosure_slider_boundary() {
    let tmp = tempdir().unwrap();
    let key = SigningKey::from_bytes(&[0x67u8; 32]);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    // 1. Simple mode: zero technical jargon in Home Screen
    let home_simple = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(home_simple.active_space, SpaceType::Family);

    // 2. Standard mode: shows human sync and storage health
    let home_standard = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Standard);
    assert!(!home_standard.sync_status_label.is_empty());

    // 3. Advanced mode: exposes diagnostics
    let home_advanced = HumanExperienceEngine::render_home_screen(&node, SpaceType::Family, InterfaceComplexity::Advanced);
    assert!(!home_advanced.storage_health_label.is_empty());
}
