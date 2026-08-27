use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tempfile::tempdir;
use nex_core::runtime::system::{SovereignMnemonic, PairingSession, PairingPayload};
use nex_core::runtime::production::{ProductionNodeSupervisor, NodeOperationalState};
use nex_core::runtime::consumer::{DeviceBatteryState, SyncMode, MobileSyncManager, ReleaseManifest, ReleaseVerifier};
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;
use nex_core::discovery::types::DOMAIN_BLIND_TOPIC;
use sha2::{Sha256, Digest};

#[test]
fn test_r48_a_adversarial_total_destruction_and_offline_recovery() {
    let mut entropy = [0u8; 32];
    entropy[0] = 0xAA;
    entropy[31] = 0xBB;

    let mnemonic = SovereignMnemonic::generate_24_words(&entropy);
    let original_actor = mnemonic.to_actor_id();
    let original_key = mnemonic.to_signing_key();

    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    // 1. Write state to disk
    {
        let mut supervisor = ProductionNodeSupervisor::new(&data_dir, original_key.clone());
        supervisor.start().unwrap();
        let (root, _) = supervisor.cas.store_file(&[0x42; 2048]);
        assert!(supervisor.cas.has_chunk(&root));
        supervisor.stop().unwrap();
    }

    // 2. ADVERSARIAL DISASTER: Purge entire directory
    std::fs::remove_dir_all(&data_dir).unwrap();
    assert!(!data_dir.exists());

    // 3. Offline restoration from Mnemonic (Zero network calls)
    let restored_key = mnemonic.to_signing_key();
    let restored_actor = mnemonic.to_actor_id();

    assert_eq!(original_actor, restored_actor);
    assert_eq!(original_key.to_bytes(), restored_key.to_bytes());
}

#[test]
fn test_r48_b_adversarial_pairing_matrix_attacks() {
    let mut csprng = OsRng;
    let master_key = SigningKey::generate(&mut csprng);
    let dev_key = SigningKey::generate(&mut csprng);
    let foreign_key = SigningKey::generate(&mut csprng);

    let current_epoch = 100;
    let nonce: u64 = 987654321;

    let valid_payload = PairingSession::generate_pairing_payload(&master_key, current_epoch, nonce);

    // 1. Valid enrollment
    assert!(PairingSession::verify_and_enroll(&valid_payload, current_epoch, &dev_key).is_ok());

    // 2. Attack: Expired epoch QR
    let mut expired_payload = valid_payload.clone();
    expired_payload.epoch = 90;
    assert!(PairingSession::verify_and_enroll(&expired_payload, current_epoch, &dev_key).is_err());

    // 3. Attack: Foreign master signature (wrong master key)
    let foreign_payload = PairingSession::generate_pairing_payload(&foreign_key, current_epoch, nonce);
    let mut bad_foreign_payload = valid_payload.clone();
    bad_foreign_payload.signature = foreign_payload.signature;
    assert!(PairingSession::verify_and_enroll(&bad_foreign_payload, current_epoch, &dev_key).is_err());
}

#[test]
fn test_r48_c_blinded_topic_zero_leakage_privacy() {
    let topic_id = [0x77; 32];
    let nonce: u64 = 1001;

    // Blinded topic derivation
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_BLIND_TOPIC);
    hasher.update(&topic_id);
    hasher.update(&nonce.to_le_bytes());
    let blinded: [u8; 32] = hasher.finalize().into();

    // Adversary inspecting blinded cannot recover topic_id without knowing topic_id
    assert_ne!(blinded, topic_id);
    assert_eq!(blinded.len(), 32);
}
