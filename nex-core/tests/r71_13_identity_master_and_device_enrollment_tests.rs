use std::collections::BTreeSet;
use ed25519_dalek::SigningKey;
use nex_core::identity::master::NexMasterIdentity;
use nex_core::identity::types::KeyType;
use nex_core::identity::verifier::{derive_actor_id, verify_device_certificate_with_crl};

#[test]
fn test_r71_13_a_master_identity_creation_and_actor_id() {
    let seed = [0x11u8; 32];
    let master = NexMasterIdentity::from_seed(&seed);

    let expected_actor = derive_actor_id(KeyType::Ed25519, &master.master_verifying_key.to_bytes());
    assert_eq!(master.root_actor_id, expected_actor);
}

#[test]
fn test_r71_13_b_issue_valid_device_certificate() {
    let master_seed = [0x01u8; 32];
    let master = NexMasterIdentity::from_seed(&master_seed);

    let device_key = SigningKey::from_bytes(&[0x02u8; 32]);
    let device_pk = device_key.verifying_key().to_bytes();

    let cert = master.issue_device_certificate(&device_pk, 100, 500).expect("Issuance failed");

    let crl = BTreeSet::new();
    let res = verify_device_certificate_with_crl(&cert, &master.root_actor_id, 250, &crl);
    assert!(res.is_ok(), "Valid device certificate must verify against root master actor");
}

#[test]
fn test_r71_13_c_expired_device_certificate_rejection() {
    let master_seed = [0x03u8; 32];
    let master = NexMasterIdentity::from_seed(&master_seed);

    let device_key = SigningKey::from_bytes(&[0x04u8; 32]);
    let device_pk = device_key.verifying_key().to_bytes();

    let cert = master.issue_device_certificate(&device_pk, 100, 200).unwrap();

    let crl = BTreeSet::new();
    // Attempt verification at epoch 300 > 200
    let res = verify_device_certificate_with_crl(&cert, &master.root_actor_id, 300, &crl);
    assert!(res.is_err(), "Expired device certificate must fail");
}

#[test]
fn test_r71_13_d_device_crl_revocation_enforcement() {
    let master_seed = [0x05u8; 32];
    let master = NexMasterIdentity::from_seed(&master_seed);

    let device_key = SigningKey::from_bytes(&[0x06u8; 32]);
    let device_pk = device_key.verifying_key().to_bytes();

    let cert = master.issue_device_certificate(&device_pk, 100, 500).unwrap();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pk);

    let mut crl = BTreeSet::new();
    master.revoke_device(&mut crl, device_actor);

    let res = verify_device_certificate_with_crl(&cert, &master.root_actor_id, 250, &crl);
    assert!(res.is_err(), "Revoked device on CRL must be rejected");
}

#[test]
fn test_r71_13_e_multi_device_enrollment_under_single_root() {
    let master_seed = [0x07u8; 32];
    let master = NexMasterIdentity::from_seed(&master_seed);

    let d1_pk = SigningKey::from_bytes(&[0x08u8; 32]).verifying_key().to_bytes();
    let d2_pk = SigningKey::from_bytes(&[0x09u8; 32]).verifying_key().to_bytes();

    let c1 = master.issue_device_certificate(&d1_pk, 100, 500).unwrap();
    let c2 = master.issue_device_certificate(&d2_pk, 100, 500).unwrap();

    let crl = BTreeSet::new();
    assert!(verify_device_certificate_with_crl(&c1, &master.root_actor_id, 250, &crl).is_ok());
    assert!(verify_device_certificate_with_crl(&c2, &master.root_actor_id, 250, &crl).is_ok());

    assert_eq!(c1.master_actor_id, master.root_actor_id);
    assert_eq!(c2.master_actor_id, master.root_actor_id);
    assert_ne!(c1.device_actor_id, c2.device_actor_id);
}

#[test]
fn test_r71_13_f_unrelated_root_cert_rejection() {
    let m1 = NexMasterIdentity::from_seed(&[0x0Au8; 32]);
    let m2 = NexMasterIdentity::from_seed(&[0x0Bu8; 32]);

    let d_pk = SigningKey::from_bytes(&[0x0Cu8; 32]).verifying_key().to_bytes();
    let c1 = m1.issue_device_certificate(&d_pk, 100, 500).unwrap();

    let crl = BTreeSet::new();
    // Verify against M2's root actor ID (must fail RootIssuerMismatch)
    let res = verify_device_certificate_with_crl(&c1, &m2.root_actor_id, 250, &crl);
    assert!(res.is_err());
}
