use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::consumer::{
    DeviceBatteryState, SyncMode, MobileSyncManager,
    ReleaseManifest, ReleaseVerifier, I18nEngine
};

#[test]
fn test_r47_b_mobile_battery_sync_throttling() {
    // 1. Charging -> Always Full
    assert_eq!(MobileSyncManager::determine_sync_mode(DeviceBatteryState::Charging(15)), SyncMode::Full);

    // 2. High Battery (>50%) -> Full
    assert_eq!(MobileSyncManager::determine_sync_mode(DeviceBatteryState::Discharging(80)), SyncMode::Full);

    // 3. Medium Battery (20-50%) -> MetadataOnly (Sparse CAS conservation)
    assert_eq!(MobileSyncManager::determine_sync_mode(DeviceBatteryState::Discharging(35)), SyncMode::MetadataOnly);

    // 4. Low Battery (<20%) -> Paused
    assert_eq!(MobileSyncManager::determine_sync_mode(DeviceBatteryState::Discharging(12)), SyncMode::Paused);
}

#[test]
fn test_r47_c_release_manifest_signing_and_tamper_rejection() {
    let mut csprng = OsRng;
    let release_key = SigningKey::generate(&mut csprng);
    let release_pubkey = release_key.verifying_key().to_bytes();

    let mut hashes = BTreeMap::new();
    hashes.insert("nex-linux-x86_64".to_string(), [0xAA; 32]);
    hashes.insert("nex-windows-x64.exe".to_string(), [0xBB; 32]);

    let manifest = ReleaseManifest {
        version: "v1.0.0".to_string(),
        binary_hashes: hashes,
    };

    // 1. Sign manifest with release key
    let sig = ReleaseVerifier::sign_manifest(&manifest, &release_key);

    // 2. Client verifies genuine release manifest
    let is_valid = ReleaseVerifier::verify_manifest(&manifest, &sig, &release_pubkey).unwrap();
    assert!(is_valid);

    // 3. Tampered binary hash in manifest -> Verification MUST FAIL
    let mut tampered_manifest = manifest.clone();
    tampered_manifest.binary_hashes.insert("nex-linux-x86_64".to_string(), [0xFF; 32]);
    let res_tampered = ReleaseVerifier::verify_manifest(&tampered_manifest, &sig, &release_pubkey);
    assert!(res_tampered.is_err(), "Tampered release manifest must be rejected");
}

#[test]
fn test_r47_d_accessibility_and_i18n_localization() {
    let mut i18n = I18nEngine::new("en");

    let mut en_catalog = BTreeMap::new();
    en_catalog.insert("drive.title".to_string(), "Sovereign Drive".to_string());
    en_catalog.insert("chat.send".to_string(), "Send Encrypted".to_string());
    i18n.register_catalog("en", en_catalog);

    let mut ar_catalog = BTreeMap::new();
    ar_catalog.insert("drive.title".to_string(), "محرك الأقراص السيادي".to_string());
    i18n.register_catalog("ar", ar_catalog);

    // 1. English translation
    assert_eq!(i18n.translate("drive.title", "Drive"), "Sovereign Drive");
    assert!(!i18n.is_right_to_left());

    // 2. Switch to Arabic & check RTL
    i18n.active_locale = "ar".to_string();
    assert_eq!(i18n.translate("drive.title", "Drive"), "محرك الأقراص السيادي");
    assert!(i18n.is_right_to_left());

    // 3. Fallback translation for missing key in Arabic
    assert_eq!(i18n.translate("chat.send", "Send"), "Send");
}
