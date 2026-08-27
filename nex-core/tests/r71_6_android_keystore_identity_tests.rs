use std::collections::BTreeSet;
use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use nex_core::runtime::mobile::AndroidKeystoreBroker;
use nex_core::identity::types::{DeviceCertificate, KeyType};
use nex_core::identity::verifier::{derive_actor_id, DOMAIN_DEVICE_CERT};

#[test]
fn test_r71_6_a_keystore_key_generation_and_signing() {
    let device_seed = [0x55u8; 32];
    let broker = AndroidKeystoreBroker::generate_in_keystore(&device_seed);

    let message = b"Sovereign Android Client Transaction Payload";
    let sig_bytes = broker.sign_payload(message).expect("Signing failed");
    assert_eq!(sig_bytes.len(), 64);
}

#[test]
fn test_r71_6_b_valid_device_certificate_verification() {
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed = [0x02u8; 32];
    let mut broker = AndroidKeystoreBroker::generate_in_keystore(&device_seed);
    let device_pubkey = broker.device_verifying_key.to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pubkey);

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&device_actor);
    hasher.update(&100u64.to_le_bytes()); // not_before
    hasher.update(&500u64.to_le_bytes()); // expires_at
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let root_sig = root_key.sign(&cert_hash).to_bytes();

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: root_sig.to_vec(),
    };

    broker.set_certificate(cert);

    let crl = BTreeSet::new();
    let auth_res = broker.verify_device_authorization(&root_actor, 250, &crl);
    assert!(auth_res.is_ok(), "Valid device cert must verify");
}

#[test]
fn test_r71_6_c_expired_device_certificate_rejection() {
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed = [0x04u8; 32];
    let mut broker = AndroidKeystoreBroker::generate_in_keystore(&device_seed);
    let device_pubkey = broker.device_verifying_key.to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pubkey);

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&device_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&200u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let root_sig = root_key.sign(&cert_hash).to_bytes();

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 200,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: root_sig.to_vec(),
    };
    broker.set_certificate(cert);

    let crl = BTreeSet::new();
    // Current epoch 300 > 200 (Expired)
    let auth_res = broker.verify_device_authorization(&root_actor, 300, &crl);
    assert!(auth_res.is_err(), "Expired device cert must be rejected");
}

#[test]
fn test_r71_6_d_revoked_device_certificate_via_crl() {
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed = [0x06u8; 32];
    let mut broker = AndroidKeystoreBroker::generate_in_keystore(&device_seed);
    let device_pubkey = broker.device_verifying_key.to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pubkey);

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&device_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&500u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let root_sig = root_key.sign(&cert_hash).to_bytes();

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: root_sig.to_vec(),
    };
    broker.set_certificate(cert);

    // Device is placed on CRL
    let mut crl = BTreeSet::new();
    crl.insert(device_actor);

    // Attempt authorization
    let auth_res = broker.verify_device_authorization(&root_actor, 200, &crl);
    assert!(auth_res.is_err(), "Revoked device on CRL must be immediately rejected");
}

#[test]
fn test_r71_6_e_device_key_mismatch_detection() {
    let root_seed = [0x07u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed_1 = [0x08u8; 32];
    let device_seed_2 = [0x09u8; 32];
    let mut broker = AndroidKeystoreBroker::generate_in_keystore(&device_seed_1);
    let other_device_key = SigningKey::from_bytes(&device_seed_2).verifying_key().to_bytes();
    let other_device_actor = derive_actor_id(KeyType::Ed25519, &other_device_key);

    // Certificate crafted for other_device_actor, but loaded into broker 1
    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: other_device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: vec![0u8; 64],
    };
    broker.set_certificate(cert);

    let crl = BTreeSet::new();
    let auth_res = broker.verify_device_authorization(&root_actor, 200, &crl);
    assert!(auth_res.is_err(), "Key mismatch between hardware key and cert must fail");
}

#[test]
fn test_r71_6_f_root_actor_preservation_across_device_revocation() {
    let root_seed = [0x0Au8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let stolen_device_seed = [0x0Bu8; 32];
    let valid_device_seed = [0x0Cu8; 32];

    let stolen_broker = AndroidKeystoreBroker::generate_in_keystore(&stolen_device_seed);
    let mut valid_broker = AndroidKeystoreBroker::generate_in_keystore(&valid_device_seed);

    let stolen_pubkey = stolen_broker.device_verifying_key.to_bytes();
    let stolen_actor = derive_actor_id(KeyType::Ed25519, &stolen_pubkey);

    let valid_pubkey = valid_broker.device_verifying_key.to_bytes();
    let valid_actor = derive_actor_id(KeyType::Ed25519, &valid_pubkey);

    // Create valid cert for valid_broker
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&valid_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&500u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let sig = root_key.sign(&cert_hash).to_bytes();

    valid_broker.set_certificate(DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: valid_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: sig.to_vec(),
    });

    // Revoke stolen device
    let mut crl = BTreeSet::new();
    crl.insert(stolen_actor);

    // Valid device remains functional under the same root ActorID
    let res = valid_broker.verify_device_authorization(&root_actor, 200, &crl);
    assert!(res.is_ok());
}
