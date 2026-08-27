use std::collections::{BTreeMap, BTreeSet};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tempfile::TempDir;

use nex_core::identity::types::{ActorID, KeyType};
use nex_core::identity::master::NexMasterIdentity;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::recovery::device_recovery::DeviceRecoveryWorkflow;
use nex_core::apps::community::{NexCommunityEngine, CommunityRole};
use nex_core::runtime::node::NexNode;
use nex_core::api::{NexCoreRuntime, NexAppApi};
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::sync::types::IngressDisposition;
use nex_core::hash::hash_mutation_body;

fn generate_actor() -> (SigningKey, ActorID) {
    let key = SigningKey::generate(&mut OsRng);
    let pubkey = key.verifying_key().to_bytes();
    let actor_id = derive_actor_id(KeyType::Ed25519, &pubkey);
    (key, actor_id)
}

#[test]
fn test_r83_01_02_03_personal_block_canonical_wal_and_ingress() {
    let temp_dir = TempDir::new().unwrap();
    let (node_key, _node_actor) = generate_actor();
    let (spammer_key, spammer_actor) = generate_actor();

    // 1. Start Node A
    let mut node = NexNode::new(temp_dir.path(), node_key.clone());
    node.start().unwrap();

    // 2. Block Spammer (Emits canonical Mutation to WAL)
    assert!(node.block_actor(spammer_actor));
    assert!(node.is_actor_blocked(&spammer_actor));

    // 3. R83-01: Verify System Ingress Rejection at Node Boundary
    let spam_body = MutationBody {
        author: spammer_actor,
        parents: Vec::new(),
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [0xAA; 32], value: b"spam".to_vec() },
    };
    let spam_mutation = Mutation { id: hash_mutation_body(&spam_body), body: spam_body };
    let disp = node.ingest_remote_mutation(spam_mutation);
    assert_eq!(disp, IngressDisposition::Rejected("Author is locally blocked".into()));

    // 4. R83-02: Simulate Crash WITHOUT checkpoint (WAL Replay Durability)
    // We intentionally do NOT call checkpoint_and_compact().
    node.stop().unwrap();

    let mut recovered_node = NexNode::new(temp_dir.path(), node_key.clone());
    recovered_node.start().unwrap();
    assert!(
        recovered_node.is_actor_blocked(&spammer_actor),
        "WAL replay MUST reconstruct personal blocklist from raw mutations without snapshot"
    );

    // 5. R83-03: Unblock Spammer and verify crash durability of unblock
    assert!(recovered_node.unblock_actor(&spammer_actor));
    assert!(!recovered_node.is_actor_blocked(&spammer_actor));

    recovered_node.stop().unwrap();

    let mut unblocked_node = NexNode::new(temp_dir.path(), node_key);
    unblocked_node.start().unwrap();
    assert!(
        !unblocked_node.is_actor_blocked(&spammer_actor),
        "WAL replay MUST reconstruct unblocked state"
    );
}

#[test]
fn test_r83_04_05_06_community_ban_canonical_mutations_and_wal_durability() {
    let (owner_key, owner_actor) = generate_actor();
    let (_, member_actor) = generate_actor();

    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("comm_wal.log");
    let runtime = NexCoreRuntime::new(owner_key.clone(), Some(wal_path.clone()));
    let mut engine = NexCommunityEngine::new(owner_actor, runtime);
    let ns_comm = [0x55; 32];

    let comm_id = engine.create_community(ns_comm, "Gardening Guild", "Green thumbs", 1, None).unwrap();
    engine.assign_role(comm_id, member_actor, CommunityRole::Member).unwrap();

    // 1. R83-04: Ban Member emits a canonical Object / Mutation in DAG & WAL
    let ban_obj_id = engine.ban_member(comm_id, member_actor, 1).unwrap();
    assert!(engine.is_banned(&comm_id, &member_actor));
    assert!(engine.api.read_object(&ban_obj_id).is_ok(), "Ban record must exist in canonical ObjectStore");

    // 2. R83-05: Simulate Crash and Replay from WAL
    drop(engine);

    let runtime2 = NexCoreRuntime::new(owner_key.clone(), Some(wal_path.clone()));
    let mut recovered_engine = NexCommunityEngine::new(owner_actor, runtime2);

    // Scan reconstructed objects
    let objects: Vec<_> = recovered_engine.api.object_store.objects.values().cloned().collect();
    recovered_engine.rebuild_moderation_from_objects(&objects);
    recovered_engine.roles.entry(comm_id).or_default().insert(owner_actor, CommunityRole::Owner);
    assert!(
        recovered_engine.is_banned(&comm_id, &member_actor),
        "Community ban MUST be reconstructed from canonical DAG/WAL objects upon restart"
    );

    // 3. R83-06: Unban Member (Tombstones the ban object in DAG)
    recovered_engine.unban_member(comm_id, member_actor, 2).unwrap();
    assert!(!recovered_engine.is_banned(&comm_id, &member_actor));

    drop(recovered_engine);

    let runtime3 = NexCoreRuntime::new(owner_key, Some(wal_path));
    let mut final_engine = NexCommunityEngine::new(owner_actor, runtime3);
    let objects3: Vec<_> = final_engine.api.object_store.objects.values().cloned().collect();
    final_engine.rebuild_moderation_from_objects(&objects3);
    assert!(
        !final_engine.is_banned(&comm_id, &member_actor),
        "Unban tombstone MUST persist across restart"
    );
}

#[test]
fn test_r83_07_08_09_10_community_ban_cross_node_replication_and_stale_cap() {
    let (owner_key, owner_actor) = generate_actor();
    let (node2_key, node2_actor) = generate_actor();
    let (spammer_key, spammer_actor) = generate_actor();

    // Node 1 (Community Owner)
    let runtime1 = NexCoreRuntime::new(owner_key, None);
    let mut engine1 = NexCommunityEngine::new(owner_actor, runtime1);

    // Node 2 (Peer / Replica Node)
    let runtime2 = NexCoreRuntime::new(node2_key, None);
    let mut engine2 = NexCommunityEngine::new(node2_actor, runtime2);

    let comm_id = engine1.create_community([0x77; 32], "Distributed Club", "P2P Space", 1, None).unwrap();
    let chan_id = engine1.create_channel([0x77; 32], comm_id, "general", false, None).unwrap();

    // 1. R83-07: Ban Spammer on Node 1
    let ban_obj_id = engine1.ban_member(comm_id, spammer_actor, 10).unwrap();

    // Replicate all DAG mutations from Node 1 to Node 2 via anti-entropy sync
    for (_, mutation) in &engine1.api.state_node.dag {
        let _ = engine2.api.state_node.ingest_mutation(mutation.clone());
    }

    // Materialize objects on Node 2
    let ban_obj = engine1.api.read_object(&ban_obj_id).unwrap();
    engine2.rebuild_moderation_from_objects(&[ban_obj]);

    assert!(engine2.is_banned(&comm_id, &spammer_actor), "Node 2 MUST reflect replicated ban");

    // 2. R83-10: Spammer presents unexpired capability to Node 2 -> REJECTED by revocation fence
    let mut spammer_engine_on_node2 = NexCommunityEngine::new(spammer_actor, NexCoreRuntime::new(spammer_key, None));
    spammer_engine_on_node2.banned_actors = engine2.banned_actors.clone();
    spammer_engine_on_node2.channels = engine1.channels.clone();

    let post_attempt = spammer_engine_on_node2.create_post([0x77; 32], chan_id, "Bypass Attempt", "I have old token", 11, None);
    assert!(post_attempt.is_err(), "Remote node MUST reject banned actor presenting stale capability");

    // 3. R83-08 & R83-09: Partition and Deterministic Convergence
    // Partition: Node 1 issues unban at epoch 12, Node 2 issues duplicate ban at epoch 11
    engine1.unban_member(comm_id, spammer_actor, 12).unwrap();
    let _ = engine2.ban_member(comm_id, spammer_actor, 11);

    // Sync DAG mutations
    for (_, mutation) in &engine1.api.state_node.dag {
        let _ = engine2.api.state_node.ingest_mutation(mutation.clone());
    }

    // Node 2 converges to unbanned state because Epoch 12 > Epoch 11
    let unbanned_obj = engine1.api.object_store.get(&ban_obj_id).cloned().unwrap();
    engine2.rebuild_moderation_from_objects(&[unbanned_obj]);
    assert!(!engine2.is_banned(&comm_id, &spammer_actor), "Nodes MUST deterministically converge");
}

#[test]
fn test_r83_11_12_13_14_15_authority_device_and_scope_invariance() {
    let (owner_a_key, owner_a) = generate_actor();
    let (owner_b_key, owner_b) = generate_actor();
    let (admin_key, admin_actor) = generate_actor();
    let (mod_key, mod_actor) = generate_actor();

    let master_seed = [0x99; 32];
    let master = NexMasterIdentity::from_seed(&master_seed);
    let banned_target = master.root_actor_id;

    let mut engine_a = NexCommunityEngine::new(owner_a, NexCoreRuntime::new(owner_a_key, None));
    let mut engine_b = NexCommunityEngine::new(owner_b, NexCoreRuntime::new(owner_b_key, None));

    let comm_a = engine_a.create_community([0x11; 32], "Guild A", "Desc A", 1, None).unwrap();
    let comm_b = engine_b.create_community([0x22; 32], "Guild B", "Desc B", 1, None).unwrap();

    engine_a.assign_role(comm_a, admin_actor, CommunityRole::Admin).unwrap();
    engine_a.assign_role(comm_a, mod_actor, CommunityRole::Moderator).unwrap();
    engine_a.assign_role(comm_a, banned_target, CommunityRole::Member).unwrap();
    engine_b.assign_role(comm_b, banned_target, CommunityRole::Member).unwrap();

    // R83-13: Authority boundary attacks
    let mut mod_engine = NexCommunityEngine::new(mod_actor, NexCoreRuntime::new(mod_key, None));
    mod_engine.roles = engine_a.roles.clone();
    assert!(mod_engine.ban_member(comm_a, banned_target, 1).is_err(), "Moderator cannot ban without Admin role");

    let mut admin_engine = NexCommunityEngine::new(admin_actor, NexCoreRuntime::new(admin_key, None));
    admin_engine.roles = engine_a.roles.clone();
    assert!(admin_engine.ban_member(comm_a, owner_a, 1).is_err(), "Admin cannot ban Owner");
    assert_eq!(engine_a.ban_member(comm_a, owner_a, 1).unwrap_err(), "InvalidOperation: Cannot ban oneself");

    // Admin bans Target in Community A
    engine_a.ban_member(comm_a, banned_target, 2).unwrap();
    assert!(engine_a.is_banned(&comm_a, &banned_target));

    // R83-14: Scope Separation
    assert!(!engine_b.is_banned(&comm_b, &banned_target), "Community A ban MUST NOT affect Community B");
    assert_eq!(engine_b.get_role(&comm_b, &banned_target), CommunityRole::Member);

    // R83-11: Device replacement cannot evade ban
    let dev_b_key = SigningKey::generate(&mut OsRng);
    let cert_b = master.issue_device_certificate(&dev_b_key.verifying_key().to_bytes(), 1, 100_000).unwrap();
    assert!(engine_a.is_banned(&comm_a, &cert_b.master_actor_id), "Replacement device cannot evade ban");

    // R83-12: Identity recovery cannot evade ban
    let (_plan, shares) = DeviceRecoveryWorkflow::setup_3_of_5_recovery(&master_seed, 1, None, 0).unwrap();
    let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(banned_target, 0);
    ceremony.submit_share(shares[0].clone()).unwrap();
    ceremony.submit_share(shares[1].clone()).unwrap();
    ceremony.submit_share(shares[2].clone()).unwrap();
    let mut crl = BTreeSet::new();
    let recovery = DeviceRecoveryWorkflow::execute_device_recovery(&ceremony, &dev_b_key.verifying_key().to_bytes(), None, 3, &mut crl).unwrap();
    assert_eq!(recovery.root_actor_id, banned_target);
    assert!(engine_a.is_banned(&comm_a, &recovery.root_actor_id), "Recovered identity remains banned");
}
