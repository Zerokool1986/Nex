use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use tempfile::tempdir;
use nex_core::runtime::system::{SovereignMnemonic, PairingSession};
use nex_core::runtime::production::ProductionNodeSupervisor;
use nex_core::api::NexAppApi;
use nex_core::object::types::ObjectType;

#[test]
fn test_r46_a_onboarding_mnemonic_and_total_destruction_recovery() {
    let entropy: [u8; 32] = [0x5A; 32];

    // 1. Generate 24-word sovereign mnemonic
    let mnemonic = SovereignMnemonic::generate_24_words(&entropy);
    assert_eq!(mnemonic.words.len(), 24);

    let key1 = mnemonic.to_signing_key();
    let actor1 = mnemonic.to_actor_id();

    // 2. Setup Node A and write sovereign state
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    {
        let mut supervisor = ProductionNodeSupervisor::new(&data_dir, key1.clone());
        supervisor.start().unwrap();
        let (root, _) = supervisor.cas.store_file(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert!(supervisor.cas.has_chunk(&root));
        supervisor.stop().unwrap();
    }

    // 3. Simulate TOTAL NODE DESTRUCTION (rm -rf)
    let _ = std::fs::remove_dir_all(&data_dir);
    assert!(!data_dir.exists());

    // 4. Disaster Recovery from 24-word mnemonic phrase
    let key_recovered = mnemonic.to_signing_key();
    let actor_recovered = mnemonic.to_actor_id();

    assert_eq!(actor1, actor_recovered, "Recovered ActorID must be byte-identical to original");
    assert_eq!(key1.to_bytes(), key_recovered.to_bytes(), "Recovered SigningKey must match original");

    // 5. Initialize fresh node with recovered identity
    let mut recovered_supervisor = ProductionNodeSupervisor::new(&data_dir, key_recovered);
    recovered_supervisor.start().unwrap();
    assert_eq!(recovered_supervisor.runtime.actor_id, actor1);
    recovered_supervisor.stop().unwrap();
}

#[test]
fn test_r46_b_out_of_band_pairing_and_adversarial_rejection() {
    let master_key = SigningKey::from_bytes(&[0x11; 32]);
    let dev_key = SigningKey::from_bytes(&[0x22; 32]);

    let current_epoch = 10;
    let nonce = 123456789;

    // 1. Master generates Pairing Payload (Optical QR representation)
    let payload = PairingSession::generate_pairing_payload(&master_key, current_epoch, nonce);
    assert_eq!(payload.epoch, current_epoch);

    // 2. Device verifies and successfully enrolls
    let cert_res = PairingSession::verify_and_enroll(&payload, current_epoch, &dev_key);
    assert!(cert_res.is_ok(), "Valid pairing payload must be accepted");
    let cert = cert_res.unwrap();
    assert_eq!(cert.master_actor_id, payload.master_actor_id);

    // 3. Adversarial Rejection: Expired Epoch QR Payload
    let expired_epoch = 5;
    let expired_payload = PairingSession::generate_pairing_payload(&master_key, expired_epoch, nonce);
    let expired_res = PairingSession::verify_and_enroll(&expired_payload, current_epoch, &dev_key);
    assert!(expired_res.is_err(), "Expired pairing QR must be rejected");

    // 4. Adversarial Rejection: Signature Forgery
    let mut forged_payload = payload.clone();
    forged_payload.signature = vec![0xEE; 64];
    let forged_res = PairingSession::verify_and_enroll(&forged_payload, current_epoch, &dev_key);
    assert!(forged_res.is_err(), "Forged signature pairing QR must be rejected");
}

#[test]
fn test_r46_c_usable_system_substrate_and_dataset_stability() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let signing_key = SigningKey::from_bytes(&[0x33; 32]);

    let mut supervisor = ProductionNodeSupervisor::new(&data_dir, signing_key);
    supervisor.start().unwrap();

    let namespace = [0x77; 32];

    // 1. Drive Inode Object
    let mut meta = BTreeMap::new();
    meta.insert("path".to_string(), "/reports/q3.pdf".to_string());
    let payload = vec![0x01, 0x02, 0x03, 0x04];
    let obj_id = supervisor.runtime.create_object(namespace, ObjectType::DriveInode, meta, payload).unwrap();
    assert_ne!(obj_id, [0u8; 32]);

    // 2. Photo Media Object
    let mut photo_meta = BTreeMap::new();
    photo_meta.insert("album".to_string(), "vacation".to_string());
    let photo_id = supervisor.runtime.create_object(namespace, ObjectType::PhotoMedia, photo_meta, vec![0xAA; 4096]).unwrap();
    assert_ne!(photo_id, [0u8; 32]);

    // 3. Chat Message Object
    let mut chat_meta = BTreeMap::new();
    chat_meta.insert("channel".to_string(), "general".to_string());
    let msg_id = supervisor.runtime.create_object(namespace, ObjectType::ChatMessage, chat_meta, vec![0xBB; 256]).unwrap();
    assert_ne!(msg_id, [0u8; 32]);

    assert_eq!(supervisor.runtime.object_store.len(), 3);
    supervisor.stop().unwrap();
}
