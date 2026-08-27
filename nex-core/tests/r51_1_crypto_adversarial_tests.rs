use rand::{Rng, RngCore};
use rand::rngs::OsRng;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::transport::session::{NaspInitiator, NaspResponder};

#[test]
fn test_r51_1_a_formal_aead_aad_tamper_defense() {
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(key_a);
    let mut responder = NaspResponder::new(key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, t3) = responder.process_init(&init_msg).unwrap();
    let (confirm_msg, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();
    responder.verify_confirm(&init_msg.static_pub, &t3, &confirm_msg).unwrap();

    let valid_aad = b"UNENCRYPTED_ROUTING_HEADER_V1";
    let tampered_aad = b"UNENCRYPTED_ROUTING_HEADER_V2";
    let plaintext = b"HIGHLY_CONFIDENTIAL_PAYLOAD_512";

    // 1. Encrypt with valid AAD
    let (seq, ciphertext, mac) = initiator_keys.encrypt_with_aad(valid_aad, plaintext);

    // 2. Attack: Modify unencrypted AAD in transit
    let tamper_res = responder_keys.decrypt_with_aad(tampered_aad, seq, &ciphertext, &mac);
    assert!(tamper_res.is_err(), "Tampered AAD must cause AEAD MAC verification failure");

    // 3. Decrypt with valid AAD succeeds
    let valid_res = responder_keys.decrypt_with_aad(valid_aad, seq, &ciphertext, &mac)
        .expect("Valid AAD must decrypt cleanly");
    assert_eq!(valid_res, plaintext);
}

#[test]
fn test_r51_1_b_simultaneous_inflight_concurrent_rekeying() {
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(key_a);
    let mut responder = NaspResponder::new(key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, _) = responder.process_init(&init_msg).unwrap();
    let (_, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();

    // Initiator sends Frame 1 (Epoch 0)
    let (seq1, ct1, mac1) = initiator_keys.encrypt(b"INFLIGHT_FRAME_EPOCH_0");

    // Both peers simultaneously trigger rekey
    initiator_keys.rekey();
    responder_keys.rekey();

    // Responder receives in-flight Frame 1 (sent before Initiator rekeyed)
    // Responder uses transition buffer to decrypt gracefully
    let decrypted_inflight = responder_keys.decrypt(seq1, &ct1, &mac1)
        .expect("In-flight frame must be decrypted using transition key buffer");
    assert_eq!(decrypted_inflight, b"INFLIGHT_FRAME_EPOCH_0");

    // Initiator sends Frame 2 (Epoch 1)
    let (seq2, ct2, mac2) = initiator_keys.encrypt(b"NEW_FRAME_EPOCH_1");
    let decrypted_new = responder_keys.decrypt(seq2, &ct2, &mac2)
        .expect("New frame must be decrypted using active key");
    assert_eq!(decrypted_new, b"NEW_FRAME_EPOCH_1");
}

#[test]
fn test_r51_1_c_zero_persistence_and_reboot_nonce_collision_defense() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut initiator1 = NaspInitiator::new(static_key_a.clone());
    let mut responder1 = NaspResponder::new(static_key_b.clone());

    let init1 = initiator1.generate_init();
    let (reply1, mut resp_keys1, _) = responder1.process_init(&init1).unwrap();
    let (_, mut init_keys1) = initiator1.process_reply(&reply1).unwrap();

    let (seq1, _, _) = init_keys1.encrypt(b"MSG1");
    assert_eq!(seq1, 1);

    // Drop session keys (simulating node crash & restart)
    drop(init_keys1);
    drop(resp_keys1);

    // Re-instantiate initiator and responder with same static keys
    let mut initiator2 = NaspInitiator::new(static_key_a);
    let mut responder2 = NaspResponder::new(static_key_b);

    let init2 = initiator2.generate_init();
    let (reply2, mut resp_keys2, _) = responder2.process_init(&init2).unwrap();
    let (_, mut init_keys2) = initiator2.process_reply(&reply2).unwrap();

    // Verify fresh session keys are completely distinct (Forward Secrecy & Zero Persistence)
    assert_ne!(initiator1.ephemeral_pub, initiator2.ephemeral_pub);
    assert_ne!(init_keys2.k_tx, [0u8; 32]);

    let (seq_new, ct_new, mac_new) = init_keys2.encrypt(b"NEW_MSG_AFTER_REBOOT");
    assert_eq!(seq_new, 1); // Sequence starts cleanly at 1 without nonce reuse on fresh key
    let decrypted = resp_keys2.decrypt(seq_new, &ct_new, &mac_new).unwrap();
    assert_eq!(decrypted, b"NEW_MSG_AFTER_REBOOT");
}

#[test]
fn test_r51_1_d_cross_protocol_signature_replay_defense() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut responder = NaspResponder::new(static_key_b);

    // Attacker crafts a signature from a different domain (e.g. Capability Token)
    let foreign_payload = b"NEX/CAPABILITY_TOKEN/v1_SOME_DELEGATION_PAYLOAD";
    let foreign_sig = static_key_a.sign(foreign_payload).to_bytes().to_vec();

    let forged_init = nex_core::transport::session::NaspInit {
        ephemeral_pub: [0x55; 32],
        static_pub: static_key_a.verifying_key().to_bytes(),
        signature: foreign_sig,
    };

    let res = responder.process_init(&forged_init);
    assert!(res.is_err(), "Cross-protocol replayed signature must fail domain-separated verification");
}

#[test]
fn test_r51_1_e_truncated_transcript_relay_mitm_defense() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(static_key_a);
    let mut responder = NaspResponder::new(static_key_b);

    let mut init_msg = initiator.generate_init();
    // Attacker swaps ephemeral public key in transit
    init_msg.ephemeral_pub[15] ^= 0xAA;

    let res = responder.process_init(&init_msg);
    assert!(res.is_err(), "Modified ephemeral public key must break transcript signature verification");
}

#[test]
fn test_r51_1_f_high_throughput_cryptographic_fuzzing() {
    let mut csprng = OsRng;
    let key_a = SigningKey::generate(&mut csprng);
    let key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(key_a);
    let mut responder = NaspResponder::new(key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, _) = responder.process_init(&init_msg).unwrap();
    let (_, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();

    let mut rng = rand::thread_rng();

    for i in 0..1000u64 {
        let payload = format!("FUZZ_PAYLOAD_TEST_{}", i).into_bytes();
        let (seq, ct, mac) = initiator_keys.encrypt(&payload);

        let corrupt_type: u8 = rng.gen_range(0..4);
        match corrupt_type {
            0 => {
                // Authentic Frame: must pass
                let dec = responder_keys.decrypt(seq, &ct, &mac).expect("Authentic frame must pass");
                assert_eq!(dec, payload);
            }
            1 => {
                // Corrupted Ciphertext: must fail
                let mut bad_ct = ct.clone();
                let flip_idx = rng.gen_range(0..bad_ct.len());
                bad_ct[flip_idx] ^= 0x5A;
                assert!(responder_keys.decrypt(seq, &bad_ct, &mac).is_err());
            }
            2 => {
                // Corrupted MAC: must fail
                let mut bad_mac = mac;
                bad_mac[0] ^= 0xFF;
                assert!(responder_keys.decrypt(seq, &ct, &bad_mac).is_err());
            }
            _ => {
                // Replay / Stale Seq: must fail
                if seq > 1 {
                    assert!(responder_keys.decrypt(seq - 1, &ct, &mac).is_err());
                }
            }
        }
    }
}
