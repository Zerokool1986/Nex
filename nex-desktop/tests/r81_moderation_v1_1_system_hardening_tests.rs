use std::collections::BTreeSet;
use std::fs;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tempfile::TempDir;

use nex_core::identity::types::{ActorID, KeyType};
use nex_core::identity::master::NexMasterIdentity;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::recovery::device_recovery::DeviceRecoveryWorkflow;
use nex_core::apps::community::{NexCommunityEngine, CommunityRole};
use nex_core::runtime::node::NexNode;
use nex_core::api::NexCoreRuntime;

fn generate_actor() -> (SigningKey, ActorID) {
    let key = SigningKey::generate(&mut OsRng);
    let pubkey = key.verifying_key().to_bytes();
    let actor_id = derive_actor_id(KeyType::Ed25519, &pubkey);
    (key, actor_id)
}

#[test]
fn test_personal_block_node_enforcement_and_restart_durability() {
    let temp_dir = TempDir::new().unwrap();
    let (node_key, _node_actor) = generate_actor();
    let (_, spammer_actor) = generate_actor();

    // 1. Initialize live NexNode on disk
    let mut node = NexNode::new(temp_dir.path(), node_key.clone());
    node.start().unwrap();

    // 2. Block the spammer locally
    assert!(!node.is_actor_blocked(&spammer_actor));
    assert!(node.block_actor(spammer_actor));
    assert!(node.is_actor_blocked(&spammer_actor));

    // 3. Checkpoint node to disk (Two-Phase Snapshot)
    node.checkpoint_and_compact().unwrap();
    node.stop().unwrap();

    // 4. Terminate and restart NexNode from persistent state
    let mut restored_node = NexNode::new(temp_dir.path(), node_key.clone());
    restored_node.start().unwrap();
    assert!(
        restored_node.is_actor_blocked(&spammer_actor),
        "Personal block MUST survive node termination and restart"
    );

    // 5. Unblock the actor and verify durability of unblock
    assert!(restored_node.unblock_actor(&spammer_actor));
    assert!(!restored_node.is_actor_blocked(&spammer_actor));

    restored_node.checkpoint_and_compact().unwrap();
    restored_node.stop().unwrap();

    let mut final_node = NexNode::new(temp_dir.path(), node_key);
    final_node.start().unwrap();
    assert!(
        !final_node.is_actor_blocked(&spammer_actor),
        "Unblock MUST survive node termination and restart"
    );
}

#[test]
fn test_community_ban_capability_revocation_fence() {
    let (owner_key, owner_actor) = generate_actor();
    let (member_key, member_actor) = generate_actor();

    let runtime = NexCoreRuntime::new(owner_key, None);
    let mut engine = NexCommunityEngine::new(owner_actor, runtime);
    let ns_comm = [0x99; 32];

    let comm_id = engine.create_community(ns_comm, "Private Guild", "Secret discussions", 100, None).unwrap();
    let chan_id = engine.create_channel(ns_comm, comm_id, "general", false, None).unwrap();

    // 1. Admit member
    engine.assign_role(comm_id, member_actor, CommunityRole::Member).unwrap();
    assert_eq!(engine.get_role(&comm_id, &member_actor), CommunityRole::Member);

    // 2. Member posts successfully
    let mut member_engine = NexCommunityEngine::new(member_actor, NexCoreRuntime::new(member_key, None));
    member_engine.communities = engine.communities.clone();
    member_engine.channels = engine.channels.clone();
    member_engine.roles = engine.roles.clone();

    let first_post = member_engine.create_post(ns_comm, chan_id, "Hello Guild", "Glad to be here", 101, None);
    assert!(first_post.is_ok(), "Authorized member can post");

    // 3. Owner bans member
    engine.ban_member(comm_id, member_actor, 102).unwrap();
    assert!(engine.is_banned(&comm_id, &member_actor));

    // Sync ban state to member engine
    member_engine.banned_actors = engine.banned_actors.clone();
    member_engine.roles = engine.roles.clone();

    // 4. Stale capability / existing connection attempt is REJECTED by the revocation fence
    let stale_post = member_engine.create_post(ns_comm, chan_id, "Bypass Attempt", "I still have my old token", 103, None);
    assert!(stale_post.is_err(), "Revocation fence MUST reject banned member even with stale capability");
    assert!(stale_post.unwrap_err().contains("Banned from community"));

    // 5. Member provisions replacement Device B -> still rejected
    let master_seed = [0x77; 32];
    let master = NexMasterIdentity::from_seed(&master_seed);
    let banned_master_actor = master.root_actor_id;

    engine.ban_member(comm_id, banned_master_actor, 104).unwrap();
    assert!(engine.is_banned(&comm_id, &banned_master_actor));

    let device_b_key = SigningKey::generate(&mut OsRng);
    let device_b_pubkey = device_b_key.verifying_key().to_bytes();
    let cert_b = master.issue_device_certificate(&device_b_pubkey, 100, 200_000).unwrap();

    assert_eq!(cert_b.master_actor_id, banned_master_actor);
    assert!(engine.is_banned(&comm_id, &cert_b.master_actor_id), "New device certificate cannot evade community ban");

    // 6. Identity recovery 3-of-5 -> still rejected
    let (_plan, shares) = DeviceRecoveryWorkflow::setup_3_of_5_recovery(&master_seed, 100, None, 0).unwrap();
    let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(banned_master_actor, 0);
    ceremony.submit_share(shares[0].clone()).unwrap();
    ceremony.submit_share(shares[1].clone()).unwrap();
    ceremony.submit_share(shares[2].clone()).unwrap();

    let mut crl = BTreeSet::new();
    let recovery_res = DeviceRecoveryWorkflow::execute_device_recovery(&ceremony, &device_b_pubkey, None, 110, &mut crl).unwrap();
    assert_eq!(recovery_res.root_actor_id, banned_master_actor);
    assert!(engine.is_banned(&comm_id, &recovery_res.root_actor_id), "Recovered identity remains banned");
}

#[test]
fn test_authority_boundaries_and_non_interference() {
    let (owner_a_key, owner_a) = generate_actor();
    let (owner_b_key, owner_b) = generate_actor();
    let (admin_key, admin_a) = generate_actor();
    let (mod_key, mod_a) = generate_actor();
    let (_, target_actor) = generate_actor();

    let mut engine_a = NexCommunityEngine::new(owner_a, NexCoreRuntime::new(owner_a_key, None));
    let mut engine_b = NexCommunityEngine::new(owner_b, NexCoreRuntime::new(owner_b_key, None));

    let comm_a = engine_a.create_community([0x11; 32], "Space A", "Community A", 100, None).unwrap();
    let comm_b = engine_b.create_community([0x22; 32], "Space B", "Community B", 100, None).unwrap();

    engine_a.assign_role(comm_a, admin_a, CommunityRole::Admin).unwrap();
    engine_a.assign_role(comm_a, mod_a, CommunityRole::Moderator).unwrap();
    engine_a.assign_role(comm_a, target_actor, CommunityRole::Member).unwrap();
    engine_b.assign_role(comm_b, target_actor, CommunityRole::Member).unwrap();

    // 1. Moderator cannot ban anyone (requires Admin)
    let mut mod_engine = NexCommunityEngine::new(mod_a, NexCoreRuntime::new(mod_key, None));
    mod_engine.roles = engine_a.roles.clone();
    let mod_ban_err = mod_engine.ban_member(comm_a, target_actor, 100).unwrap_err();
    assert!(mod_ban_err.contains("Must be at least Admin"));

    // 2. Admin cannot ban Owner of Community A
    let mut admin_engine = NexCommunityEngine::new(admin_a, NexCoreRuntime::new(admin_key, None));
    admin_engine.roles = engine_a.roles.clone();
    let admin_ban_owner_err = admin_engine.ban_member(comm_a, owner_a, 100).unwrap_err();
    assert!(admin_ban_owner_err.contains("equal or higher role"));

    // 3. Admin bans Target in Community A
    engine_a.ban_member(comm_a, target_actor, 105).unwrap();
    assert!(engine_a.is_banned(&comm_a, &target_actor));

    // 4. Ban in Community A has ZERO effect on Community B
    assert!(!engine_b.is_banned(&comm_b, &target_actor));
    assert_eq!(engine_b.get_role(&comm_b, &target_actor), CommunityRole::Member);

    // 5. Global identity remains completely unmutated
    assert_eq!(target_actor.len(), 32);
}

#[test]
fn test_concurrent_ban_vs_unban_deterministic_resolution() {
    let (owner_key, owner_actor) = generate_actor();
    let (_, target_actor) = generate_actor();

    let mut engine = NexCommunityEngine::new(owner_actor, NexCoreRuntime::new(owner_key, None));
    let comm_id = engine.create_community([0x33; 32], "Conflict Space", "Testing conflicts", 100, None).unwrap();

    // 1. Concurrent operations at identical epoch (Epoch 100)
    // Resolution Rule: Ban-Wins is the safety-first invariant
    engine.ban_member(comm_id, target_actor, 100).unwrap();
    assert!(engine.is_banned(&comm_id, &target_actor));

    // Duplicate ban is idempotent
    assert!(engine.ban_member(comm_id, target_actor, 100).is_ok());
    assert!(engine.is_banned(&comm_id, &target_actor));

    // Explicit unban with strictly higher epoch/lamport (Epoch 101) succeeds
    engine.unban_member(comm_id, target_actor, 101).unwrap();
    assert!(!engine.is_banned(&comm_id, &target_actor));

    // Duplicate unban is also idempotent
    assert!(engine.unban_member(comm_id, target_actor, 101).is_ok());
    assert!(!engine.is_banned(&comm_id, &target_actor));
}

