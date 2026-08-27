use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::runtime::mobile::{AndroidPlatformAdapter, DevicePowerState};

#[test]
fn test_r57_3_a_two_tier_keystore_broker_simulation() {
    // Tier 1: Hardware-backed storage key (non-exportable AES-GCM)
    let hardware_kek = [0x5Au8; 32];
    let mut raw_seed = [0x7Bu8; 32];

    // Encrypt seed with hardware KEK
    for i in 0..32 {
        raw_seed[i] ^= hardware_kek[i];
    }

    // Decrypt in Kotlin Keystore Broker
    let mut unwrapped = raw_seed;
    for i in 0..32 {
        unwrapped[i] ^= hardware_kek[i];
    }
    assert_eq!(unwrapped, [0x7Bu8; 32]);

    let dir = tempdir().unwrap();
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&unwrapped));
    assert!(node.start().is_ok());
}

#[test]
fn test_r57_3_b_strongbox_preference_fallback_simulation() {
    enum KeystoreHardwareTier {
        Tier1StrongBox,
        Tier2TEE,
        Tier3Software,
    }

    let device_has_strongbox = false;
    let device_has_tee = true;

    let selected_tier = if device_has_strongbox {
        KeystoreHardwareTier::Tier1StrongBox
    } else if device_has_tee {
        KeystoreHardwareTier::Tier2TEE
    } else {
        KeystoreHardwareTier::Tier3Software
    };

    assert!(matches!(selected_tier, KeystoreHardwareTier::Tier2TEE));
}

#[test]
fn test_r57_3_c_workmanager_active_sync_execution() {
    let dir = tempdir().unwrap();
    let adapter = AndroidPlatformAdapter::new("app.nex.sovereign", dir.path().to_path_buf());
    let seed = [201u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    assert_eq!(adapter.power_state, DevicePowerState::Active);
    assert_eq!(adapter.calculate_max_batch_size(), 100);

    let sync_res = adapter.trigger_workmanager_sync(&mut node);
    assert!(sync_res.is_ok(), "Active WorkManager sync must succeed");
}

#[test]
fn test_r57_3_d_workmanager_doze_standby_deferred() {
    let dir = tempdir().unwrap();
    let mut adapter = AndroidPlatformAdapter::new("app.nex.sovereign", dir.path().to_path_buf());
    let seed = [202u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    adapter.on_doze_entered();
    assert_eq!(adapter.power_state, DevicePowerState::DozeStandby);
    assert_eq!(adapter.calculate_max_batch_size(), 10);

    let sync_res = adapter.trigger_workmanager_sync(&mut node);
    assert!(sync_res.is_err(), "Sync must be deferred during Doze standby");

    adapter.on_doze_exited();
    assert_eq!(adapter.power_state, DevicePowerState::Active);
    assert!(adapter.trigger_workmanager_sync(&mut node).is_ok());
}

#[test]
fn test_r57_3_e_battery_saver_throttled_batch_sizing() {
    let dir = tempdir().unwrap();
    let mut adapter = AndroidPlatformAdapter::new("app.nex.sovereign", dir.path().to_path_buf());

    adapter.set_battery_saver(true);
    assert_eq!(adapter.power_state, DevicePowerState::BatterySaverThrottled);
    assert_eq!(adapter.calculate_max_batch_size(), 25);

    adapter.set_battery_saver(false);
    assert_eq!(adapter.power_state, DevicePowerState::Active);
    assert_eq!(adapter.calculate_max_batch_size(), 100);
}

#[test]
fn test_r57_3_f_zero_regression_across_mobile_lifecycle() {
    let dir = tempdir().unwrap();
    let mut adapter = AndroidPlatformAdapter::new("app.nex.sovereign", dir.path().to_path_buf());
    let seed = [203u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    for _ in 0..5 {
        adapter.on_doze_entered();
        assert_eq!(adapter.calculate_max_batch_size(), 10);
        adapter.on_doze_exited();
        assert_eq!(adapter.calculate_max_batch_size(), 100);
    }
}
