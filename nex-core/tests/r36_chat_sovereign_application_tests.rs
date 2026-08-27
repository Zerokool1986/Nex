use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::api::NexCoreRuntime;
use nex_core::apps::chat::{NexChatEngine, ChannelType, MemberRole};
use nex_core::identity::types::KeyType;
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r36_a_chat_channel_and_message_lifecycle() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let ns = [0x91; 32];
    let mut chat = NexChatEngine::new(ns, alice_actor, runtime);

    // 1. Create Channel
    let channel_id = chat.create_channel("General", ChannelType::GroupMultiParty, vec![]).unwrap();
    assert!(chat.channels.contains_key(&channel_id));

    // 2. Send E2EE Message
    let channel_key = [0x77; 32];
    let msg_text = b"Hello Nex Decentralized Mesh!";
    let msg_id = chat.send_message(channel_id, msg_text, &channel_key, vec![], vec![], None).unwrap();

    // 3. Read & Decrypt Message
    let (msg, decrypted) = chat.read_message(&msg_id, &channel_key).unwrap();
    assert_eq!(msg.author_actor_id, alice_actor);
    assert_eq!(decrypted, msg_text);
}

#[test]
fn test_r36_c_e2ee_encryption_integrity_and_tamper_rejection() {
    let key = [0x88; 32];
    let plaintext = b"SOVEREIGN_PRIVATE_CONVERSATION";

    let encrypted = NexChatEngine::<NexCoreRuntime>::encrypt_payload(plaintext, &key);
    assert_ne!(&encrypted[..plaintext.len()], plaintext);

    // Valid decryption
    let decrypted = NexChatEngine::<NexCoreRuntime>::decrypt_payload(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);

    // Tampered ciphertext -> MAC failure
    let mut tampered = encrypted.clone();
    tampered[5] ^= 0xFF;
    assert!(NexChatEngine::<NexCoreRuntime>::decrypt_payload(&tampered, &key).is_err(), "Corrupted ciphertext must fail MAC validation");

    // Wrong decryption key -> MAC failure
    let wrong_key = [0x99; 32];
    assert!(NexChatEngine::<NexCoreRuntime>::decrypt_payload(&encrypted, &wrong_key).is_err(), "Wrong key must fail MAC validation");
}

#[test]
fn test_r36_e_non_member_rejection_and_tombstones() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let bob_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_key.verifying_key().to_bytes();
    let bob_actor = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut chat = NexChatEngine::new([0x92; 32], alice_actor, runtime);

    // Create channel with only Alice as member
    let channel_id = chat.create_channel("SecretRoom", ChannelType::Direct1to1, vec![]).unwrap();

    // Switch local actor to Bob without adding to roster -> Send must fail
    chat.local_actor_id = bob_actor;
    let channel_key = [0xAA; 32];
    let res = chat.send_message(channel_id, b"UNAUTHORIZED", &channel_key, vec![], vec![], None);
    assert!(res.is_err(), "Non-member must be rejected from posting");

    // Switch back to Alice -> Post valid message -> Delete via tombstone
    chat.local_actor_id = alice_actor;
    let msg_id = chat.send_message(channel_id, b"DELETE_ME", &channel_key, vec![], vec![], None).unwrap();
    chat.delete_message(msg_id, None).unwrap();

    // Subsequent read must fail due to tombstone
    assert!(chat.read_message(&msg_id, &channel_key).is_err(), "Tombstoned message cannot be read");
}

#[test]
fn test_r36_h_reactions_add_wins_semantics() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut chat = NexChatEngine::new([0x93; 32], alice_actor, runtime);

    let msg_id = [0x55; 32];
    chat.add_reaction(msg_id, "🔥");
    chat.add_reaction(msg_id, "🚀");

    let reactions = chat.get_reactions(&msg_id);
    assert_eq!(reactions.get("🔥"), Some(&1));
    assert_eq!(reactions.get("🚀"), Some(&1));
}
