use std::collections::BTreeSet;
use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use nex_core::runtime::desktop::DesktopKeyringBroker;
use nex_core::identity::types::{DeviceCertificate, KeyType};
use nex_core::identity::verifier::{derive_actor_id, DOMAIN_DEVICE_CERT};

#[test]
fn test_r71_11_a_keyring_secret_init_and_signing() {
    let seed = [0x11u8; 32];
    let broker = DesktopKeyringBroker::init_from_secret_seed(&seed);

    let payload = b"Desktop Transaction Signature Test";
    let sig = broker.sign_payload(payload).expect("Signing failed");
    assert_eq!(sig.len(), 64);
}

#[test]
fn test_r71_11_b_valid_device_certificate_verification() {
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed = [0x02u8; 32];
    let mut broker = DesktopKeyringBroker::init_from_secret_seed(&device_seed);
    let device_pubkey = broker.device_verifying_key.to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pubkey);

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&device_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&500u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let sig = root_key.sign(&cert_hash).to_bytes();

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: sig.to_vec(),
    };
    broker.set_certificate(cert);

    let crl = BTreeSet::new();
    let res = broker.verify_device_authorization(&root_actor, 250, &crl);
    assert!(res.is_ok(), "Valid desktop device certificate must verify");
}

#[test]
fn test_r71_11_c_expired_certificate_rejection() {
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed = [0x04u8; 32];
    let mut broker = DesktopKeyringBroker::init_from_secret_seed(&device_seed);
    let device_pubkey = broker.device_verifying_key.to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pubkey);

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&device_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&200u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let sig = root_key.sign(&cert_hash).to_bytes();

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 200,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: sig.to_vec(),
    };
    broker.set_certificate(cert);

    let crl = BTreeSet::new();
    let res = broker.verify_device_authorization(&root_actor, 300, &crl);
    assert!(res.is_err(), "Expired cert must be rejected");
}

#[test]
fn test_r71_11_d_crl_revocation_rejection() {
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed = [0x06u8; 32];
    let mut broker = DesktopKeyringBroker::init_from_secret_seed(&device_seed);
    let device_pubkey = broker.device_verifying_key.to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pubkey);

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&device_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&500u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let sig = root_key.sign(&cert_hash).to_bytes();

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: device_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: sig.to_vec(),
    };
    broker.set_certificate(cert);

    let mut crl = BTreeSet::new();
    crl.insert(device_actor);

    let res = broker.verify_device_authorization(&root_actor, 200, &crl);
    assert!(res.is_err(), "Revoked cert must be rejected");
}

#[test]
fn test_r71_11_e_keyring_device_key_mismatch() {
    let root_seed = [0x07u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_seed_1 = [0x08u8; 32];
    let device_seed_2 = [0x09u8; 32];
    let mut broker = DesktopKeyringBroker::init_from_secret_seed(&device_seed_1);
    let other_key = SigningKey::from_bytes(&device_seed_2).verifying_key().to_bytes();
    let other_actor = derive_actor_id(KeyType::Ed25519, &other_key);

    let cert = DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: other_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: vec![0u8; 64],
    };
    broker.set_certificate(cert);

    let crl = BTreeSet::new();
    let res = broker.verify_device_authorization(&root_actor, 200, &crl);
    assert!(res.is_err(), "Mismatch between keyring key and cert must fail");
}

#[test]
fn test_r71_11_f_root_actor_invariance() {
    let root_seed = [0x0Au8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let d1_seed = [0x0Bu8; 32];
    let d2_seed = [0x0Cu8; 32];

    let mut b1 = DesktopKeyringBroker::init_from_secret_seed(&d1_seed);
    let b2 = DesktopKeyringBroker::init_from_secret_seed(&d2_seed);

    let d1_actor = derive_actor_id(KeyType::Ed25519, &b1.device_verifying_key.to_bytes());
    let d2_actor = derive_actor_id(KeyType::Ed25519, &b2.device_verifying_key.to_bytes());

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&root_actor);
    hasher.update(&d1_actor);
    hasher.update(&100u64.to_le_bytes());
    hasher.update(&500u64.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();
    let sig = root_key.sign(&cert_hash).to_bytes();

    b1.set_certificate(DeviceCertificate {
        master_actor_id: root_actor,
        device_actor_id: d1_actor,
        not_before_epoch: 100,
        expires_at_epoch: 500,
        master_pubkey: Some(root_pubkey.to_vec()),
        signature: sig.to_vec(),
    });

    let mut crl = BTreeSet::new();
    crl.insert(d2_actor); // Revoke device 2

    // Device 1 remains valid under the root ActorID
    let res = b1.verify_device_authorization(&root_actor, 200, &crl);
    assert!(res.is_ok());
}
