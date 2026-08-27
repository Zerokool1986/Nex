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
    let mut engine = NexCommunityEngine::new(alice_actor, runtime);
    let ns = [0xA1; 32];

    // 1. Create Community
    let comm_id = engine.create_community(ns, "Rust Builders", "Decentralized Rust Developers", 1, None).unwrap();
    assert!(engine.communities.contains_key(&comm_id));

    // 2. Create Channels
    let chan_general = engine.create_channel(ns, comm_id, "general", false, None).unwrap();
    let chan_announcements = engine.create_channel(ns, comm_id, "announcements", true, None).unwrap();

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
    let mut engine = NexCommunityEngine::new(alice_actor, runtime);
    let ns = [0xA2; 32];

    let comm_id = engine.create_community(ns, "Sovereign Tech", "Decentralized Governance", 1, None).unwrap();
    let chan_announcements = engine.create_channel(ns, comm_id, "announcements", true, None).unwrap();

    // Assign Bob as regular Member
    engine.assign_role(comm_id, bob_actor, CommunityRole::Member).unwrap();

    // Bob attempts to create a channel -> Must fail
    engine.local_actor_id = bob_actor;
    let res = engine.create_channel(ns, comm_id, "spam", false, None);
    assert!(res.is_err(), "Standard member must be rejected from creating channels");

    // Alice promotes Bob to Moderator/Admin -> Bob can now create channel
    engine.local_actor_id = alice_actor;
    engine.assign_role(comm_id, bob_actor, CommunityRole::Admin).unwrap();

    engine.local_actor_id = bob_actor;
    let chan_admin = engine.create_channel(ns, comm_id, "admin-notes", false, None).unwrap();
    assert!(engine.channels.contains_key(&chan_admin));
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
    let mut engine = NexCommunityEngine::new(alice_actor, runtime);
    let ns = [0xA3; 32];

    let comm_id = engine.create_community(ns, "Nex Core", "Core Discussion", 1, None).unwrap();
    let chan_id = engine.create_channel(ns, comm_id, "dev", false, None).unwrap();
    engine.assign_role(comm_id, bob_actor, CommunityRole::Member).unwrap();

    // Alice creates discussion post
    let post_id = engine.create_post(ns, chan_id, "RFC: ZK Fast Sync", "RFC Proposal details", 1, None).unwrap();

    // Bob creates reply to post
    engine.local_actor_id = bob_actor;
    let reply_1 = engine.create_reply(ns, post_id, None, "Looks great!", 1, None).unwrap();

    // Alice creates nested reply to Bob's reply
    engine.local_actor_id = alice_actor;
    let _reply_2 = engine.create_reply(ns, post_id, Some(reply_1), "Thanks Bob!", 1, None).unwrap();

    let reply_count = engine.replies.values().filter(|r| r.post_id == post_id).count();
    assert_eq!(reply_count, 2);

    // Alice (Moderator/Admin/Owner) locks the thread
    engine.lock_post(comm_id, post_id).unwrap();

    // Bob attempts to reply to locked thread -> Fails
    engine.local_actor_id = bob_actor;
    let res_locked = engine.create_reply(ns, post_id, None, "Another comment", 2, None);
    assert!(res_locked.is_err(), "Replies must be rejected on locked threads");
}

#[test]
fn test_r37_d_pinning_and_moderator_tombstones() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut engine = NexCommunityEngine::new(alice_actor, runtime);
    let ns = [0xA4; 32];

    let comm_id = engine.create_community(ns, "Governance", "Town Square", 1, None).unwrap();
    let chan_id = engine.create_channel(ns, comm_id, "rules", false, None).unwrap();

    let post_id = engine.create_post(ns, chan_id, "Community Guidelines", "Be respectful.", 1, None).unwrap();

    // Alice pins post
    engine.pin_post(comm_id, post_id).unwrap();
    assert!(engine.posts.get(&post_id).unwrap().is_pinned);

    // Add reactions
    engine.add_reaction(post_id, "👍");
    let reactions = engine.get_reactions(&post_id);
    assert_eq!(reactions.get("👍"), Some(&1));

    // Delete post via tombstone
    engine.moderate_tombstone_post(comm_id, post_id, None).unwrap();
    assert!(!engine.posts.contains_key(&post_id));
}
