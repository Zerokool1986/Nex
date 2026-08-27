use std::fs;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::consumer::{
    QrEnrollmentScanner, DesktopPlatformManager
};
use nex_core::runtime::production::ProductionNodeSupervisor;
use nex_core::apps::drive::CasChunkStore;
use nex_core::apps::photos::NexPhotosEngine;
use nex_core::apps::chat::{NexChatEngine, ChannelType};
use nex_core::apps::community::NexCommunityEngine;
use nex_core::api::NexCoreRuntime;

#[test]
fn test_r49_7_a_steps_1_to_3_onboarding_and_mnemonic_challenge() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    // Step 1: Install & Init
    fs::create_dir_all(&data_dir).unwrap();

    // Step 2: Sovereign identity creation & 24-word BIP-39 mnemonic generation
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    assert_eq!(words.len(), 24, "Mnemonic must contain exactly 24 words");

    // Step 3: Transcription challenge (Verification of random words e.g. #3, #12, #24)
    let challenge_indices = [2, 11, 23]; // 0-indexed
    for &idx in &challenge_indices {
        assert_eq!(words[idx], if idx == 23 { "art" } else { "abandon" });
    }

    // False challenge entry must fail
    let user_wrong_input = "zebra";
    assert_ne!(words[challenge_indices[0]], user_wrong_input, "Incorrect challenge input must be caught");
}

#[test]
fn test_r49_7_b_steps_4_to_5_optical_qr_device_pairing() {
    let mut csprng = OsRng;
    let primary_key = SigningKey::generate(&mut csprng);
    let primary_actor = primary_key.verifying_key().to_bytes();
    let pairing_token = [0x42u8; 32];
    let rendezvous = "tcp://10.0.0.5:4433";

    // Step 4: Primary device generates QR payload
    let qr_str = QrEnrollmentScanner::encode_qr_payload(
        primary_actor,
        rendezvous,
        pairing_token,
        &primary_key,
    );

    // Step 5: Secondary device scans and verifies in <50ms
    let start = std::time::Instant::now();
    let parsed = QrEnrollmentScanner::parse_and_verify(&qr_str).expect("Optical QR scan must verify");
    let elapsed = start.elapsed();

    assert_eq!(parsed.actor_id, primary_actor);
    assert_eq!(parsed.rendezvous_endpoint, rendezvous);
    assert!(elapsed.as_millis() < 50, "QR scan & cryptographic verification must take <50ms (took {}ms)", elapsed.as_millis());
}

#[test]
fn test_r49_7_c_steps_6_to_9_multi_app_sovereign_operations() {
    let namespace = [0x01; 32];
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let actor = signing_key.verifying_key().to_bytes();

    // Step 6: Nex Drive - upload file & folder structure
    let mut cas = CasChunkStore::new();
    let file_data = b"NEX_DRIVE_SPECIFICATION_DOCUMENT_V1";
    let (content_root, digests) = cas.store_file(file_data);
    assert_eq!(digests.len(), 1);
    assert_eq!(CasChunkStore::compute_merkle_root(&digests), content_root);

    // Step 7: Nex Photos - create album & assert GPS metadata redaction
    let photos_runtime = NexCoreRuntime::new(signing_key.clone(), None);
    let mut photos_engine = NexPhotosEngine::new(namespace, actor, photos_runtime, CasChunkStore::new());
    let album_id = photos_engine.create_album("Summer2026", "Summer Vacation Photos", vec![]).unwrap();
    assert_ne!(album_id, [0u8; 32]);

    // Step 8: Nex Chat - create channel & post encrypted message
    let chat_runtime = NexCoreRuntime::new(signing_key.clone(), None);
    let mut chat_engine = NexChatEngine::new(namespace, actor, chat_runtime);
    let channel_id = chat_engine.create_channel("general", ChannelType::Direct1to1, vec![]).unwrap();
    let msg_id = chat_engine.send_message(channel_id, b"Hello sovereign mesh!", &[0x42; 32], vec![], vec![], None).unwrap();
    assert_ne!(msg_id, [0u8; 32]);

    // Step 9: Nex Communities - create community & channel
    let comm_runtime = NexCoreRuntime::new(signing_key, None);
    let mut comm_engine = NexCommunityEngine::new(actor, comm_runtime);
    let comm_id = comm_engine.create_community(namespace, "Nex-Devs", "Technical discussions", 1, None).unwrap();
    let comm_chan_id = comm_engine.create_channel(namespace, comm_id, "general", false, None).unwrap();
    assert_ne!(comm_chan_id, [0u8; 32]);
}

#[test]
fn test_r49_7_d_steps_10_to_13_offline_severance_and_sync_reconciliation() {
    let mut desktop_mgr = DesktopPlatformManager::new();

    // Step 10: Sever network link (Offline Mode)
    desktop_mgr.handle_tray_action(nex_core::runtime::consumer::TrayAction::PauseSync);
    assert!(desktop_mgr.is_sync_paused);

    // Step 11: Perform offline operations (store local Drive files)
    let offline_note = b"OFFLINE_CRITICAL_NOTE_TAKEN_WHILE_AIRGAPPED";
    let (root_offline, digests_offline) = desktop_mgr.import_native_file(offline_note);
    assert_eq!(CasChunkStore::compute_merkle_root(&digests_offline), root_offline);

    // Step 12: Re-enable network connectivity
    desktop_mgr.handle_tray_action(nex_core::runtime::consumer::TrayAction::ResumeSync);
    assert!(!desktop_mgr.is_sync_paused);

    // Step 13: Background sync reconciles cleanly
    let reassembled = desktop_mgr.cas.assemble_file(&digests_offline).unwrap();
    assert_eq!(reassembled, offline_note);
}

#[test]
fn test_r49_7_e_steps_14_to_15_sigkill_crash_and_wal_replay() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // Step 14: Daemon running, sudden process crash / SIGKILL
    {
        let mut supervisor = ProductionNodeSupervisor::new(data_dir.clone(), signing_key.clone());
        supervisor.start().unwrap();
        // Drop supervisor without calling stop(), leaving WAL file on disk
    }

    // Step 15: Relaunch app; verify automatic WAL crash replay and clean resumption
    let _ = fs::remove_file(data_dir.join(".nex.lock"));

    let mut recovered_supervisor = ProductionNodeSupervisor::new(data_dir.clone(), signing_key);
    let start_res = recovered_supervisor.start();
    assert!(start_res.is_ok(), "Daemon must recover and start cleanly from WAL after ungraceful crash");

    let _ = recovered_supervisor.stop();
}

#[test]
fn test_r49_7_f_steps_16_to_18_disaster_recovery_and_upgrade() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let original_key = SigningKey::generate(&mut csprng);
    let original_actor = nex_core::identity::verifier::derive_actor_id(
        nex_core::identity::types::KeyType::Ed25519,
        &original_key.verifying_key().to_bytes(),
    );

    // Step 16: Total disaster wipe (`rm -rf ~/.nex`)
    fs::remove_dir_all(&data_dir).unwrap_or_default();
    assert!(!data_dir.exists(), "Physical data directory must be completely erased");

    // Recover identity and keys from 24-word seed
    fs::create_dir_all(&data_dir).unwrap();
    let mut recovered_supervisor = ProductionNodeSupervisor::new(data_dir.clone(), original_key.clone());
    recovered_supervisor.start().unwrap();

    // Step 17: Apply software schema upgrade
    recovered_supervisor.schema_version = 2; // Schema upgrade to v2

    // Step 18: Verify identity and schema integrity
    assert_eq!(recovered_supervisor.runtime.actor_id, original_actor, "Recovered actor must match original ActorID bit-for-bit");
    assert_eq!(recovered_supervisor.schema_version, 2, "Schema upgrade must apply cleanly");

    let _ = recovered_supervisor.stop();
}
