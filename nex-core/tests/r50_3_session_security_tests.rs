use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use nex_core::transport::session::{NaspInitiator, NaspResponder};

#[test]
fn test_r50_3_a_mutual_3way_handshake_and_key_derivation() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(static_key_a);
    let mut responder = NaspResponder::new(static_key_b);

    // 1. Message 1: NASP_INIT (A -> B)
    let init_msg = initiator.generate_init();

    // 2. Message 2: NASP_REPLY (B -> A)
    let (reply_msg, mut responder_keys, t3) = responder.process_init(&init_msg)
        .expect("Responder must accept valid init message");

    // 3. Message 3: NASP_CONFIRM (A -> B)
    let (confirm_msg, mut initiator_keys) = initiator.process_reply(&reply_msg)
        .expect("Initiator must accept valid reply message");

    // 4. Verify confirm on responder
    responder.verify_confirm(&init_msg.static_pub, &t3, &confirm_msg)
        .expect("Responder must verify confirm message");

    // 5. Symmetric key match validation
    assert_eq!(initiator_keys.k_tx, responder_keys.k_rx, "Initiator TX key must match Responder RX key");
    assert_eq!(initiator_keys.k_rx, responder_keys.k_tx, "Initiator RX key must match Responder TX key");
    assert_eq!(initiator_keys.k_mac_tx, responder_keys.k_mac_rx, "Initiator MAC TX must match Responder MAC RX");
    assert_eq!(initiator_keys.k_mac_rx, responder_keys.k_mac_tx, "Initiator MAC RX must match Responder MAC TX");
    assert_eq!(initiator_keys.k_rekey, responder_keys.k_rekey, "Both peers must derive identical rekey secret");
}

#[test]
fn test_r50_3_b_transcript_tampering_defense() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(static_key_a);
    let mut responder = NaspResponder::new(static_key_b);

    let mut init_msg = initiator.generate_init();

    // Attack: Mutate 1 byte of ephemeral public key in init
    init_msg.ephemeral_pub[0] ^= 0xFF;
    let tamper_res = responder.process_init(&init_msg);
    assert!(tamper_res.is_err(), "Responder must reject tampered init frame with signature failure");
}

#[test]
fn test_r50_3_c_rogue_key_identity_forgery_defense() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);
    let rogue_key = SigningKey::generate(&mut csprng);

    let mut rogue_initiator = NaspInitiator::new(rogue_key);
    let mut responder = NaspResponder::new(static_key_b);

    let mut init_msg = rogue_initiator.generate_init();
    // Claim to be Actor A, but signed with rogue key
    init_msg.static_pub = static_key_a.verifying_key().to_bytes();

    let res = responder.process_init(&init_msg);
    assert!(res.is_err(), "Responder must detect signature mismatch and reject rogue identity");
}

#[test]
fn test_r50_3_d_frame_confidentiality_and_mac_integrity() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(static_key_a);
    let mut responder = NaspResponder::new(static_key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, t3) = responder.process_init(&init_msg).unwrap();
    let (confirm_msg, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();
    responder.verify_confirm(&init_msg.static_pub, &t3, &confirm_msg).unwrap();

    let plaintext = b"CONFIDENTIAL_SOVEREIGN_SYNCHRONIZATION_PAYLOAD_1024_BYTES".to_vec();

    // 1. Initiator Encrypts
    let (seq, ciphertext, mac) = initiator_keys.encrypt(&plaintext);
    assert_ne!(ciphertext, plaintext, "Ciphertext must be distinct from plaintext");

    // 2. Responder Decrypts
    let decrypted = responder_keys.decrypt(seq, &ciphertext, &mac)
        .expect("Responder must decrypt valid ciphertext");
    assert_eq!(decrypted, plaintext, "Decrypted payload must match original plaintext");

    // 3. Attack: Corrupt single byte in ciphertext
    let mut corrupted_ciphertext = ciphertext.clone();
    corrupted_ciphertext[0] ^= 0x01;
    let tamper_decrypt = responder_keys.decrypt(seq + 1, &corrupted_ciphertext, &mac);
    assert!(tamper_decrypt.is_err(), "Tampered ciphertext must fail Poly1305/HMAC verification");
}

#[test]
fn test_r50_3_e_anti_replay_sequence_counter() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(static_key_a);
    let mut responder = NaspResponder::new(static_key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, _) = responder.process_init(&init_msg).unwrap();
    let (_, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();

    // Send Frame 1
    let (seq1, ct1, mac1) = initiator_keys.encrypt(b"FRAME_1");
    responder_keys.decrypt(seq1, &ct1, &mac1).unwrap();

    // Send Frame 2
    let (seq2, ct2, mac2) = initiator_keys.encrypt(b"FRAME_2");
    responder_keys.decrypt(seq2, &ct2, &mac2).unwrap();

    // Replay Frame 1
    let replay_res = responder_keys.decrypt(seq1, &ct1, &mac1);
    assert!(replay_res.is_err(), "Replayed frame 1 must be rejected by monotonic sequence check");
}

#[test]
fn test_r50_3_f_session_rekeying_and_forward_secrecy() {
    let mut csprng = OsRng;
    let static_key_a = SigningKey::generate(&mut csprng);
    let static_key_b = SigningKey::generate(&mut csprng);

    let mut initiator = NaspInitiator::new(static_key_a);
    let mut responder = NaspResponder::new(static_key_b);

    let init_msg = initiator.generate_init();
    let (reply_msg, mut responder_keys, _) = responder.process_init(&init_msg).unwrap();
    let (_, mut initiator_keys) = initiator.process_reply(&reply_msg).unwrap();

    let old_tx_key = initiator_keys.k_tx;

    // Rekey both peers
    initiator_keys.rekey();
    responder_keys.rekey();

    assert_ne!(initiator_keys.k_tx, old_tx_key, "Rekeyed key must differ from old key");
    assert_eq!(initiator_keys.k_tx, responder_keys.k_rx);

    // Verify post-rekey message exchange
    let (seq, ct, mac) = initiator_keys.encrypt(b"POST_REKEY_PAYLOAD");
    let decrypted = responder_keys.decrypt(seq, &ct, &mac).unwrap();
    assert_eq!(decrypted, b"POST_REKEY_PAYLOAD");
}
