use std::collections::BTreeSet;
use rand::rngs::OsRng;
use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use nex_core::identity::types::{ActorID, KeyType, DeviceCertificate, AuthorizationError};
use nex_core::identity::verifier::{derive_actor_id, verify_device_certificate, verify_device_certificate_with_crl, DOMAIN_DEVICE_CERT};

fn generate_ed25519_keypair() -> (SigningKey, Vec<u8>, ActorID) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let pubkey_bytes = verifying_key.to_bytes().to_vec();
    let actor_id = derive_actor_id(KeyType::Ed25519, &pubkey_bytes);
    (signing_key, pubkey_bytes, actor_id)
}

#[test]
fn test_r69_4_a_valid_device_certificate_verification() {
    let (master_sk, master_pk, master_actor) = generate_ed25519_keypair();
    let (_device_sk, _device_pk, device_actor) = generate_ed25519_keypair();

    let not_before: u64 = 100;
    let expires_at: u64 = 1000;

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&master_actor);
    hasher.update(&device_actor);
    hasher.update(&not_before.to_le_bytes());
    hasher.update(&expires_at.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();

    let signature = master_sk.sign(&cert_hash).to_bytes().to_vec();

    let cert = DeviceCertificate {
        master_actor_id: master_actor,
        device_actor_id: device_actor,
        not_before_epoch: not_before,
        expires_at_epoch: expires_at,
        master_pubkey: Some(master_pk),
        signature,
    };

    let result = verify_device_certificate(&cert, &master_actor, 500);
    assert!(result.is_ok(), "Valid device certificate must pass verification");
}

#[test]
fn test_r69_4_b_temporal_validity_bounds() {
    let (master_sk, master_pk, master_actor) = generate_ed25519_keypair();
    let (_device_sk, _device_pk, device_actor) = generate_ed25519_keypair();

    let not_before: u64 = 200;
    let expires_at: u64 = 800;

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&master_actor);
    hasher.update(&device_actor);
    hasher.update(&not_before.to_le_bytes());
    hasher.update(&expires_at.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();

    let signature = master_sk.sign(&cert_hash).to_bytes().to_vec();

    let cert = DeviceCertificate {
        master_actor_id: master_actor,
        device_actor_id: device_actor,
        not_before_epoch: not_before,
        expires_at_epoch: expires_at,
        master_pubkey: Some(master_pk),
        signature,
    };

    // Before not_before (epoch 150)
    assert!(matches!(
        verify_device_certificate(&cert, &master_actor, 150),
        Err(AuthorizationError::NotYetValid { .. })
    ));

    // After expires_at (epoch 850)
    assert!(matches!(
        verify_device_certificate(&cert, &master_actor, 850),
        Err(AuthorizationError::ExpiredCapability { .. })
    ));
}

#[test]
fn test_r69_4_c_signature_tamper_rejection() {
    let (master_sk, master_pk, master_actor) = generate_ed25519_keypair();
    let (_device_sk, _device_pk, device_actor) = generate_ed25519_keypair();

    let not_before: u64 = 100;
    let expires_at: u64 = 1000;

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&master_actor);
    hasher.update(&device_actor);
    hasher.update(&not_before.to_le_bytes());
    hasher.update(&expires_at.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();

    let mut signature = master_sk.sign(&cert_hash).to_bytes().to_vec();
    // Tamper single bit
    signature[0] ^= 0x01;

    let cert = DeviceCertificate {
        master_actor_id: master_actor,
        device_actor_id: device_actor,
        not_before_epoch: not_before,
        expires_at_epoch: expires_at,
        master_pubkey: Some(master_pk),
        signature,
    };

    assert!(matches!(
        verify_device_certificate(&cert, &master_actor, 500),
        Err(AuthorizationError::SignatureInvalid)
    ));
}

#[test]
fn test_r69_4_d_crl_revocation_instant_invalidation() {
    let (master_sk, master_pk, master_actor) = generate_ed25519_keypair();
    let (_device_sk, _device_pk, device_actor) = generate_ed25519_keypair();

    let not_before: u64 = 100;
    let expires_at: u64 = 1000;

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&master_actor);
    hasher.update(&device_actor);
    hasher.update(&not_before.to_le_bytes());
    hasher.update(&expires_at.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();

    let signature = master_sk.sign(&cert_hash).to_bytes().to_vec();

    let cert = DeviceCertificate {
        master_actor_id: master_actor,
        device_actor_id: device_actor,
        not_before_epoch: not_before,
        expires_at_epoch: expires_at,
        master_pubkey: Some(master_pk),
        signature,
    };

    let mut crl: BTreeSet<ActorID> = BTreeSet::new();
    assert!(verify_device_certificate_with_crl(&cert, &master_actor, 500, &crl).is_ok());

    // Add device to revocation list
    crl.insert(device_actor);
    assert!(matches!(
        verify_device_certificate_with_crl(&cert, &master_actor, 500, &crl),
        Err(AuthorizationError::CertificateInvalid)
    ));
}

#[test]
fn test_r69_4_e_master_actor_id_mismatch() {
    let (master_sk, master_pk, master_actor) = generate_ed25519_keypair();
    let (_imposter_sk, _imposter_pk, imposter_actor) = generate_ed25519_keypair();
    let (_device_sk, _device_pk, device_actor) = generate_ed25519_keypair();

    let not_before: u64 = 100;
    let expires_at: u64 = 1000;

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DEVICE_CERT);
    hasher.update(&master_actor);
    hasher.update(&device_actor);
    hasher.update(&not_before.to_le_bytes());
    hasher.update(&expires_at.to_le_bytes());
    let cert_hash: [u8; 32] = hasher.finalize().into();

    let signature = master_sk.sign(&cert_hash).to_bytes().to_vec();

    let cert = DeviceCertificate {
        master_actor_id: master_actor,
        device_actor_id: device_actor,
        not_before_epoch: not_before,
        expires_at_epoch: expires_at,
        master_pubkey: Some(master_pk),
        signature,
    };

    // Verify against imposter actor ID
    assert!(matches!(
        verify_device_certificate(&cert, &imposter_actor, 500),
        Err(AuthorizationError::RootIssuerMismatch)
    ));
}

#[test]
fn test_r69_4_f_multi_device_isolated_delegations() {
    let (master_sk, master_pk, master_actor) = generate_ed25519_keypair();
    let (_phone_sk, _phone_pk, phone_actor) = generate_ed25519_keypair();
    let (_laptop_sk, _laptop_pk, laptop_actor) = generate_ed25519_keypair();

    let make_cert = |dev_actor: ActorID| {
        let not_before: u64 = 100;
        let expires_at: u64 = 1000;
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_DEVICE_CERT);
        hasher.update(&master_actor);
        hasher.update(&dev_actor);
        hasher.update(&not_before.to_le_bytes());
        hasher.update(&expires_at.to_le_bytes());
        let cert_hash: [u8; 32] = hasher.finalize().into();
        let signature = master_sk.sign(&cert_hash).to_bytes().to_vec();
        DeviceCertificate {
            master_actor_id: master_actor,
            device_actor_id: dev_actor,
            not_before_epoch: not_before,
            expires_at_epoch: expires_at,
            master_pubkey: Some(master_pk.clone()),
            signature,
        }
    };

    let phone_cert = make_cert(phone_actor);
    let laptop_cert = make_cert(laptop_actor);

    let mut crl: BTreeSet<ActorID> = BTreeSet::new();

    // Revoke only phone
    crl.insert(phone_actor);

    assert!(verify_device_certificate_with_crl(&phone_cert, &master_actor, 500, &crl).is_err());
    assert!(verify_device_certificate_with_crl(&laptop_cert, &master_actor, 500, &crl).is_ok(), "Revoking phone must not affect laptop");
}
