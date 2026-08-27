use ed25519_dalek::SigningKey;
use nex_core::identity::master::NexMasterIdentity;
use nex_core::identity::pairing::{PairingPayload, PairingSessionInitiator, compute_sas_code};

#[test]
fn test_r71_14_a_qr_uri_encoding_and_decoding_roundtrip() {
    let payload = PairingPayload {
        session_id: [0x11; 16],
        ephemeral_pubkey: [0x22; 32],
        nonce: [0x33; 16],
        expires_at_epoch: 12345678,
        rendezvous: "192.168.1.50:9876".to_string(),
    };

    let uri = payload.encode_qr_uri();
    assert!(uri.starts_with("nex://pair/v1?"));

    let decoded = PairingPayload::decode_qr_uri(&uri).expect("Decoding failed");
    assert_eq!(decoded, payload);
}

#[test]
fn test_r71_14_b_sas_code_derivation_determinism() {
    let pk_a = [0xAA; 32];
    let pk_b = [0xBB; 32];
    let nonce_a = [0x12; 16];
    let nonce_b = [0x34; 16];

    let sas1 = compute_sas_code(&pk_a, &pk_b, &nonce_a, &nonce_b);
    let sas2 = compute_sas_code(&pk_a, &pk_b, &nonce_a, &nonce_b);

    assert_eq!(sas1, sas2);
    assert!(sas1 < 1_000_000, "SAS must be a 6-digit number < 1,000,000");
}

#[test]
fn test_r71_14_c_pairing_ceremony_success() {
    let master = NexMasterIdentity::from_seed(&[0x01u8; 32]);
    let initiator = PairingSessionInitiator::new([0x02; 16], &[0x03u8; 32], [0x04; 16], 1000);

    let candidate_device_key = SigningKey::from_bytes(&[0x05u8; 32]);
    let candidate_device_pk = candidate_device_key.verifying_key().to_bytes();

    let candidate_ephemeral_key = SigningKey::from_bytes(&[0x06u8; 32]);
    let candidate_ephemeral_pk = candidate_ephemeral_key.verifying_key().to_bytes();
    let candidate_nonce = [0x07; 16];

    let expected_sas = compute_sas_code(
        &initiator.local_ephemeral_key.verifying_key().to_bytes(),
        &candidate_ephemeral_pk,
        &initiator.local_nonce,
        &candidate_nonce,
    );

    let (sas, cert) = initiator.complete_pairing(
        &master,
        &candidate_device_pk,
        &candidate_ephemeral_pk,
        &candidate_nonce,
        expected_sas,
        500,
        10000,
    ).expect("Pairing failed");

    assert_eq!(sas, expected_sas);
    assert_eq!(cert.master_actor_id, master.root_actor_id);
}

#[test]
fn test_r71_14_d_sas_mismatch_rejection() {
    let master = NexMasterIdentity::from_seed(&[0x01u8; 32]);
    let initiator = PairingSessionInitiator::new([0x02; 16], &[0x03u8; 32], [0x04; 16], 1000);

    let candidate_device_pk = [0x05; 32];
    let candidate_ephemeral_pk = [0x06; 32];
    let candidate_nonce = [0x07; 16];

    let wrong_sas = 999999;
    let res = initiator.complete_pairing(
        &master,
        &candidate_device_pk,
        &candidate_ephemeral_pk,
        &candidate_nonce,
        wrong_sas,
        500,
        10000,
    );

    assert!(res.is_err(), "SAS code mismatch must reject enrollment");
}

#[test]
fn test_r71_14_e_expired_pairing_session_rejection() {
    let master = NexMasterIdentity::from_seed(&[0x01u8; 32]);
    let initiator = PairingSessionInitiator::new([0x02; 16], &[0x03u8; 32], [0x04; 16], 1000); // Exp = 1000

    let candidate_device_pk = [0x05; 32];
    let candidate_ephemeral_pk = [0x06; 32];
    let candidate_nonce = [0x07; 16];

    let expected_sas = compute_sas_code(
        &initiator.local_ephemeral_key.verifying_key().to_bytes(),
        &candidate_ephemeral_pk,
        &initiator.local_nonce,
        &candidate_nonce,
    );

    // Current epoch 1500 > 1000
    let res = initiator.complete_pairing(
        &master,
        &candidate_device_pk,
        &candidate_ephemeral_pk,
        &candidate_nonce,
        expected_sas,
        1500,
        10000,
    );

    assert!(res.is_err(), "Expired pairing session must be rejected");
}

#[test]
fn test_r71_14_f_malformed_qr_uri_rejection() {
    let bad_uris = vec![
        "https://example.com/pair",
        "nex://wrong_scheme",
        "nex://pair/v1?sid=bad_hex",
        "nex://pair/v1?sid=0102&pk=0304", // incomplete params
    ];

    for u in bad_uris {
        assert!(PairingPayload::decode_qr_uri(u).is_err());
    }
}
