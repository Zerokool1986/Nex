use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::api::NexCoreRuntime;
use nex_core::apps::community::{NexCommunityEngine, CommunityRole};
use nex_core::identity::types::KeyType;
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r37_a_community_creation_and_channel_lifecycle() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let ns = [0xA1; 32];
    let mut engine = NexCommunityEngine::new(ns, alice_actor, runtime);

    // 1. Create Community
    let comm_id = engine.create_community("Rust Builders", "Decentralized Rust Developers").unwrap();
    assert!(engine.communities.contains_key(&comm_id));

    // 2. Create Channels
    let chan_general = engine.create_channel(comm_id, "general", false).unwrap();
    let chan_announcements = engine.create_channel(comm_id, "announcements", true).unwrap();

    assert!(engine.channels.contains_key(&chan_general));
    assert!(engine.channels.contains_key(&chan_announcements));
}

#[test]
fn test_r37_b_role_hierarchy_and_announcement_enforcement() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let bob_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_key.verifying_key().to_bytes();
    let bob_actor = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut engine = NexCommunityEngine::new([0xA2; 32], alice_actor, runtime);

    let comm_id = engine.create_community("Sovereign Tech", "Decentralized Governance").unwrap();
    let chan_announcements = engine.create_channel(comm_id, "announcements", true).unwrap();

    // Assign Bob as regular Member
    engine.assign_role(comm_id, bob_actor, CommunityRole::Member).unwrap();

    // Bob attempts to post in announcements -> Must fail
    engine.local_actor_id = bob_actor;
    let res = engine.create_post(comm_id, chan_announcements, "Spam", "Buy coins", vec![], None);
    assert!(res.is_err(), "Standard member must be rejected from posting in announcements");

    // Alice promotes Bob to Moderator -> Bob can now post in announcements
    engine.local_actor_id = alice_actor;
    engine.assign_role(comm_id, bob_actor, CommunityRole::Moderator).unwrap();

    engine.local_actor_id = bob_actor;
    let post_id = engine.create_post(comm_id, chan_announcements, "Update", "Release v1", vec![], None).unwrap();
    assert!(engine.posts.contains_key(&post_id));
}

#[test]
fn test_r37_c_threaded_replies_and_thread_locking() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let bob_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_key.verifying_key().to_bytes();
    let bob_actor = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut engine = NexCommunityEngine::new([0xA3; 32], alice_actor, runtime);

    let comm_id = engine.create_community("Nex Core", "Core Discussion").unwrap();
    let chan_id = engine.create_channel(comm_id, "dev", false).unwrap();
    engine.assign_role(comm_id, bob_actor, CommunityRole::Member).unwrap();

    // Alice creates discussion post
    let post_id = engine.create_post(comm_id, chan_id, "RFC: ZK Fast Sync", "RFC Proposal details", vec![], None).unwrap();

    // Bob creates reply to post
    engine.local_actor_id = bob_actor;
    let reply_1 = engine.create_reply(post_id, None, "Looks great!").unwrap();

    // Alice creates nested reply to Bob's reply
    engine.local_actor_id = alice_actor;
    let reply_2 = engine.create_reply(post_id, Some(reply_1), "Thanks Bob!").unwrap();

    assert_eq!(engine.replies.get(&post_id).unwrap().len(), 2);

    // Alice (Moderator) locks the thread
    engine.lock_thread(post_id, true).unwrap();

    // Bob attempts to reply to locked thread -> Fails
    engine.local_actor_id = bob_actor;
    let res_locked = engine.create_reply(post_id, None, "Another comment");
    assert!(res_locked.is_err(), "Replies must be rejected on locked threads");
}

#[test]
fn test_r37_d_pinning_and_moderator_tombstones() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut engine = NexCommunityEngine::new([0xA4; 32], alice_actor, runtime);

    let comm_id = engine.create_community("Governance", "Town Square").unwrap();
    let chan_id = engine.create_channel(comm_id, "rules", false).unwrap();

    let post_id = engine.create_post(comm_id, chan_id, "Community Guidelines", "Be respectful.", vec![], None).unwrap();

    // Alice pins post
    engine.pin_post(post_id, true).unwrap();
    assert!(engine.posts.get(&post_id).unwrap().is_pinned);

    // Add reactions
    engine.add_reaction(post_id, "👍");
    let reactions = engine.get_reactions(&post_id);
    assert_eq!(reactions.get("👍"), Some(&1));

    // Delete post via tombstone
    engine.tombstone_post(post_id, None).unwrap();
    assert!(!engine.posts.contains_key(&post_id));
}
