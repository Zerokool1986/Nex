use std::time::Instant;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::consumer::{
    AndroidLifecycleManager, AndroidPowerState, NetworkInterfaceType,
    SyncMode, AndroidKeyStoreEnclave, QrEnrollmentScanner
};

#[test]
fn test_r49_5_a_keystore_tee_seed_protection() {
    let mut csprng = OsRng;
    let raw_mnemonic_seed = SigningKey::generate(&mut csprng).to_bytes();
    let tee_hardware_key = [0x5Au8; 32];

    // 1. Wrap 256-bit mnemonic seed inside hardware-backed TEE envelope
    let encrypted_seed = AndroidKeyStoreEnclave::wrap_seed(&raw_mnemonic_seed, &tee_hardware_key);
    assert_ne!(encrypted_seed, raw_mnemonic_seed, "Encrypted seed in KeyStore must not match raw entropy");

    // 2. Unwrap seed from KeyStore
    let recovered_seed = AndroidKeyStoreEnclave::unwrap_seed(&encrypted_seed, &tee_hardware_key);
    assert_eq!(recovered_seed, raw_mnemonic_seed, "KeyStore unwrapped seed must match original 256-bit seed bit-for-bit");

    // 3. Wrong hardware key must fail to recover original seed
    let wrong_hardware_key = [0x99u8; 32];
    let corrupt_recovery = AndroidKeyStoreEnclave::unwrap_seed(&encrypted_seed, &wrong_hardware_key);
    assert_ne!(corrupt_recovery, raw_mnemonic_seed);
}

#[test]
fn test_r49_5_b_optical_qr_enrollment_scan_and_verify() {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let actor_id = signing_key.verifying_key().to_bytes();
    let rendezvous = "tcp://192.168.1.100:4433";
    let pairing_token = [0x77u8; 32];

    // 1. Node generates QR payload string
    let qr_string = QrEnrollmentScanner::encode_qr_payload(
        actor_id,
        rendezvous,
        pairing_token,
        &signing_key,
    );

    // 2. Mobile camera captures and parses QR string with latency measurement
    let start_time = Instant::now();
    let parsed_payload = QrEnrollmentScanner::parse_and_verify(&qr_string)
        .expect("Legitimate optical QR string must parse and verify cleanly");
    let elapsed = start_time.elapsed();

    assert_eq!(parsed_payload.actor_id, actor_id);
    assert_eq!(parsed_payload.rendezvous_endpoint, rendezvous);
    assert_eq!(parsed_payload.pairing_token, pairing_token);
    assert!(elapsed.as_millis() < 50, "QR parse and cryptographic verification must complete in under 50ms (took {}ms)", elapsed.as_millis());

    // 3. Adversarial test: Tampered rendezvous endpoint in QR payload must be rejected
    let mut tampered_qr = qr_string.clone();
    tampered_qr = tampered_qr.replace("192.168.1.100", "192.168.1.222");
    let tamper_result = QrEnrollmentScanner::parse_and_verify(&tampered_qr);
    assert!(tamper_result.is_err(), "Tampered QR payload must be rejected with signature invalid");
}

#[test]
fn test_r49_5_c_android_doze_mode_battery_throttling() {
    let mut manager = AndroidLifecycleManager::new();

    // 1. Interactive active state -> Full sync
    let mode = manager.handle_power_transition(AndroidPowerState::Interactive);
    assert_eq!(mode, SyncMode::Full);

    // 2. Deep Doze Mode (screen off, device stationary) -> Paused, zero wake-locks
    let doze_mode = manager.handle_power_transition(AndroidPowerState::DozeMode);
    assert_eq!(doze_mode, SyncMode::Paused);

    // 3. Battery Saver at 10% (< 20%) -> Paused
    let crit_mode = manager.handle_power_transition(AndroidPowerState::BatterySaver(10));
    assert_eq!(crit_mode, SyncMode::Paused);

    // 4. Battery Saver at 35% (>= 20%) -> MetadataOnly (CAS downloads deferred)
    let meta_mode = manager.handle_power_transition(AndroidPowerState::BatterySaver(35));
    assert_eq!(meta_mode, SyncMode::MetadataOnly);

    // 5. Device plugged into charger -> Full sync resumed
    let charge_mode = manager.handle_power_transition(AndroidPowerState::Charging(40));
    assert_eq!(charge_mode, SyncMode::Full);
}

#[test]
fn test_r49_5_d_foreground_service_bulk_cas_sync() {
    let mut manager = AndroidLifecycleManager::new();

    // With foreground service active, Doze Mode must NOT pause bulk sync
    manager.is_foreground_service_active = true;
    let mode = manager.handle_power_transition(AndroidPowerState::DozeMode);
    assert_eq!(mode, SyncMode::Full, "Foreground Service notification must exempt bulk CAS transfer from Doze throttling");

    // Stopping foreground service returns to Doze pause
    manager.is_foreground_service_active = false;
    let paused_mode = manager.handle_power_transition(AndroidPowerState::DozeMode);
    assert_eq!(paused_mode, SyncMode::Paused);
}

#[test]
fn test_r49_5_e_wifi_to_cellular_roaming() {
    let mut manager = AndroidLifecycleManager::new();
    assert_eq!(manager.network_type, NetworkInterfaceType::Wifi);
    assert!(manager.active_socket_connected);

    // 1. Walk out of WiFi range -> Disconnected
    let res_disc = manager.handle_network_roaming(NetworkInterfaceType::Disconnected);
    assert!(!res_disc);
    assert!(!manager.active_socket_connected);

    // 2. Cellular LTE/5G connects -> Automatic socket renewal without crashing
    let res_cell = manager.handle_network_roaming(NetworkInterfaceType::Cellular);
    assert!(res_cell);
    assert!(manager.active_socket_connected, "Cellular connection must restore active socket state");
}

#[test]
fn test_r49_5_f_boot_completed_autonomous_recovery() {
    let mut manager = AndroidLifecycleManager::new();
    manager.is_node_running = false;

    // Simulate Android OS broadcast: BOOT_COMPLETED
    let boot_success = manager.handle_boot_completed();
    assert!(boot_success);
    assert!(manager.is_node_running, "Nex daemon must start autonomously upon device boot completion");
    assert!(manager.active_socket_connected);
}
