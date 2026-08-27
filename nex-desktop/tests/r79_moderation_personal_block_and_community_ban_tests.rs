use std::collections::BTreeSet;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

use nex_core::identity::types::{ActorID, KeyType};
use nex_core::identity::master::NexMasterIdentity;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::blocklist::PersonalBlocklist;
use nex_core::identity::recovery::device_recovery::DeviceRecoveryWorkflow;
use nex_core::apps::community::{NexCommunityEngine, CommunityRole};
use nex_core::api::NexCoreRuntime;

fn generate_actor() -> (SigningKey, ActorID) {
    let key = SigningKey::generate(&mut OsRng);
    let pubkey = key.verifying_key().to_bytes();
    let actor_id = derive_actor_id(KeyType::Ed25519, &pubkey);
    (key, actor_id)
}

#[test]
fn test_personal_block_direct_lifecycle() {
    let mut blocklist = PersonalBlocklist::new();
    let (_, alice) = generate_actor();
    let (_, bob_spammer) = generate_actor();

    // 1. Block Actor Bob
    assert!(blocklist.block_actor(bob_spammer));
    assert!(blocklist.is_blocked(&bob_spammer));
    assert!(!blocklist.is_blocked(&alice));

    // 2. Direct interaction check
    let can_alice_interact = !blocklist.is_blocked(&alice);
    let can_bob_interact = !blocklist.is_blocked(&bob_spammer);
    assert!(can_alice_interact, "Alice must be allowed");
    assert!(!can_bob_interact, "Blocked Bob must be rejected locally");

    // 3. Bob's global identity remains completely valid (unmutated)
    assert_eq!(bob_spammer.len(), 32);

    // 4. Unblock Bob
    assert!(blocklist.unblock_actor(&bob_spammer));
    assert!(!blocklist.is_blocked(&bob_spammer));
    let can_bob_interact_now = !blocklist.is_blocked(&bob_spammer);
    assert!(can_bob_interact_now, "Bob can interact again after unblocking");
}

#[test]
fn test_community_scoped_ban_and_unban_lifecycle() {
    let (owner_key, owner_actor) = generate_actor();
    let (bob_key, member_bob) = generate_actor();
    let runtime = NexCoreRuntime::new(owner_key, None);
    let mut engine = NexCommunityEngine::new(owner_actor, runtime);

    let ns_comm = [0xCC; 32];
    let comm_id = engine.create_community(ns_comm, "Sovereign Gardeners", "All about gardening", 100, None).unwrap();
    let chan_id = engine.create_channel(ns_comm, comm_id, "general", false, None).unwrap();

    // 1. Admit Bob as Member
    engine.assign_role(comm_id, member_bob, CommunityRole::Member).unwrap();
    assert_eq!(engine.get_role(&comm_id, &member_bob), CommunityRole::Member);

    // 2. Owner bans Bob
    engine.ban_member(comm_id, member_bob, 105).unwrap();
    assert!(engine.is_banned(&comm_id, &member_bob));
    assert_eq!(engine.get_role(&comm_id, &member_bob), CommunityRole::Guest, "Banned member falls back to Guest/None");

    // 3. Cannot re-assign role while banned
    let reassign_res = engine.assign_role(comm_id, member_bob, CommunityRole::Member);
    assert!(reassign_res.is_err(), "Cannot assign role to banned actor");

    // 4. Banned Bob cannot create posts in the channel
    let bob_runtime = NexCoreRuntime::new(bob_key, None);
    let mut bob_engine = NexCommunityEngine::new(member_bob, bob_runtime);
    bob_engine.communities = engine.communities.clone();
    bob_engine.channels = engine.channels.clone();
    bob_engine.roles = engine.roles.clone();
    bob_engine.banned_actors = engine.banned_actors.clone();

    let post_res = bob_engine.create_post(ns_comm, chan_id, "Spam Title", "Buy spam now!", 106, None);
    assert!(post_res.is_err(), "Banned member cannot post");
    assert!(post_res.unwrap_err().contains("Banned from community"));

    // 5. Owner unbans Bob
    engine.unban_member(comm_id, member_bob, 110).unwrap();
    assert!(!engine.is_banned(&comm_id, &member_bob));

    // 6. Bob can now be legitimately re-admitted
    engine.assign_role(comm_id, member_bob, CommunityRole::Member).unwrap();
    assert_eq!(engine.get_role(&comm_id, &member_bob), CommunityRole::Member);
}

#[test]
fn test_community_ban_scope_independence() {
    let (owner_a_key, owner_a) = generate_actor();
    let (owner_b_key, owner_b) = generate_actor();
    let (_, charlie) = generate_actor();

    let runtime_a = NexCoreRuntime::new(owner_a_key, None);
    let runtime_b = NexCoreRuntime::new(owner_b_key, None);
    let mut engine_a = NexCommunityEngine::new(owner_a, runtime_a);
    let mut engine_b = NexCommunityEngine::new(owner_b, runtime_b);

    let comm_a = engine_a.create_community([0xAA; 32], "Community A", "Desc A", 100, None).unwrap();
    let comm_b = engine_b.create_community([0xBB; 32], "Community B", "Desc B", 100, None).unwrap();

    engine_a.assign_role(comm_a, charlie, CommunityRole::Member).unwrap();
    engine_b.assign_role(comm_b, charlie, CommunityRole::Member).unwrap();

    // Owner A bans Charlie from Community A
    engine_a.ban_member(comm_a, charlie, 105).unwrap();

    assert!(engine_a.is_banned(&comm_a, &charlie), "Charlie must be banned in A");
    assert!(!engine_b.is_banned(&comm_b, &charlie), "Charlie must NOT be banned in B");
    assert_eq!(engine_b.get_role(&comm_b, &charlie), CommunityRole::Member, "Charlie remains active member in B");
}

#[test]
fn test_banned_actor_device_separation_and_recovery_invariance() {
    // 1. Establish Master Identity for Banned Actor
    let master_seed = [0x55; 32];
    let master = NexMasterIdentity::from_seed(&master_seed);
    let banned_root_actor = master.root_actor_id;

    let (comm_owner_key, comm_owner) = generate_actor();
    let runtime = NexCoreRuntime::new(comm_owner_key, None);
    let mut engine = NexCommunityEngine::new(comm_owner, runtime);
    let comm_id = engine.create_community([0x77; 32], "Exclusive Space", "Private", 100, None).unwrap();

    // Owner bans root Actor
    engine.ban_member(comm_id, banned_root_actor, 100).unwrap();
    assert!(engine.is_banned(&comm_id, &banned_root_actor));

    // 2. Banned Actor provisions a new replacement device (Device B)
    let device_b_key = SigningKey::generate(&mut OsRng);
    let device_b_pubkey = device_b_key.verifying_key().to_bytes();
    let cert_b = master.issue_device_certificate(&device_b_pubkey, 100, 200_000).unwrap();

    // Certificate links Device B to banned_root_actor
    assert_eq!(cert_b.master_actor_id, banned_root_actor);

    // Banned check against master ActorID remains strictly true
    assert!(engine.is_banned(&comm_id, &cert_b.master_actor_id), "New device cannot bypass community ban");

    // 3. 3-of-5 Recovery of identity also preserves the ban
    let (_plan, shares) = DeviceRecoveryWorkflow::setup_3_of_5_recovery(&master_seed, 100, None, 0).unwrap();
    let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(banned_root_actor, 0);
    ceremony.submit_share(shares[0].clone()).unwrap();
    ceremony.submit_share(shares[1].clone()).unwrap();
    ceremony.submit_share(shares[2].clone()).unwrap();

    let mut crl = BTreeSet::new();
    let recovery_res = DeviceRecoveryWorkflow::execute_device_recovery(&ceremony, &device_b_pubkey, None, 110, &mut crl).unwrap();

    assert_eq!(recovery_res.root_actor_id, banned_root_actor);
    assert!(engine.is_banned(&comm_id, &recovery_res.root_actor_id), "Recovered identity remains banned");
}

#[test]
fn test_moderation_adversarial_edge_cases() {
    let (owner_key, owner) = generate_actor();
    let (admin_key, admin) = generate_actor();
    let (mod_key, moderator) = generate_actor();
    let (member_key, regular_member) = generate_actor();
    let (_, target) = generate_actor();

    let runtime = NexCoreRuntime::new(owner_key, None);
    let mut engine = NexCommunityEngine::new(owner, runtime);
    let comm_id = engine.create_community([0x99; 32], "Test Community", "Desc", 100, None).unwrap();

    engine.assign_role(comm_id, admin, CommunityRole::Admin).unwrap();
    engine.assign_role(comm_id, moderator, CommunityRole::Moderator).unwrap();
    engine.assign_role(comm_id, regular_member, CommunityRole::Member).unwrap();

    // 1. Regular member cannot ban anyone
    let member_runtime = NexCoreRuntime::new(member_key, None);
    let mut member_engine = NexCommunityEngine::new(regular_member, member_runtime);
    member_engine.roles = engine.roles.clone();
    let ban_err = member_engine.ban_member(comm_id, target, 100).unwrap_err();
    assert!(ban_err.contains("Must be at least Admin"));

    // 2. Moderator cannot ban anyone (must be at least Admin)
    let mod_runtime = NexCoreRuntime::new(mod_key, None);
    let mut mod_engine = NexCommunityEngine::new(moderator, mod_runtime);
    mod_engine.roles = engine.roles.clone();
    let mod_ban_err = mod_engine.ban_member(comm_id, target, 100).unwrap_err();
    assert!(mod_ban_err.contains("Must be at least Admin"));

    // 3. Admin cannot ban another Admin or the Owner
    let admin_runtime = NexCoreRuntime::new(admin_key, None);
    let mut admin_engine = NexCommunityEngine::new(admin, admin_runtime);
    admin_engine.roles = engine.roles.clone();
    let admin_ban_owner_err = admin_engine.ban_member(comm_id, owner, 100).unwrap_err();
    assert!(admin_ban_owner_err.contains("equal or higher role"));

    // 4. Owner cannot ban themselves
    let self_ban_err = engine.ban_member(comm_id, owner, 100).unwrap_err();
    assert_eq!(self_ban_err, "InvalidOperation: Cannot ban oneself");

    // 5. Idempotent unban of unbanned actor does not panic
    let unban_res = engine.unban_member(comm_id, target, 100);
    assert!(unban_res.is_ok(), "Unbanning non-banned actor is safe and idempotent");
}
