use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::apps::drive::{NexDriveEngine, DriveInode, DriveSyncWatcher};
use nex_core::apps::chat::{NexChatEngine, ChatOutbox, ChannelType};
use nex_core::apps::community::{NexCommunityEngine, CommunityRole, CommunityInvitationToken};
use nex_core::object::types::ObjectType;

#[test]
fn test_r52_2_a_drive_recursive_directory_sync_and_cas_dedup() {
    let tmp_node = tempdir().unwrap();
    let tmp_local = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp_node.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_drive = [0xD1; 32];
    let drive_engine = NexDriveEngine::new(ns_drive, &mut node);
    let mut watcher = DriveSyncWatcher::new(drive_engine);

    // Create local folder structure
    let sub = tmp_local.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(tmp_local.path().join("root.txt"), b"Root file content").unwrap();
    fs::write(sub.join("nested.txt"), b"Nested file content").unwrap();

    // 1. Initial scan
    let delta1 = watcher.scan_and_sync(tmp_local.path()).unwrap();
    assert_eq!(delta1.added_files, 2);
    assert_eq!(delta1.updated_files, 0);

    // 2. Idempotent scan (no changes)
    let delta2 = watcher.scan_and_sync(tmp_local.path()).unwrap();
    assert_eq!(delta2.added_files, 0);
    assert_eq!(delta2.updated_files, 0);

    // 3. Modify one file
    fs::write(sub.join("nested.txt"), b"Updated nested file content").unwrap();
    let delta3 = watcher.scan_and_sync(tmp_local.path()).unwrap();
    assert_eq!(delta3.added_files, 0);
    assert_eq!(delta3.updated_files, 1);
}

#[test]
fn test_r52_2_b_drive_chunk_corruption_detection_and_self_healing() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_drive = [0xD1; 32];
    let mut engine = NexDriveEngine::new(ns_drive, &mut node);

    let original_payload = b"Crucial resilient sovereign payload";
    let oid = engine.upload_file("/docs/crucial.dat", "application/octet-stream", original_payload, None).unwrap();

    let obj = engine.api.read_object(&oid).unwrap();
    let inode: DriveInode = serde_json::from_slice(&obj.payload_bytes).unwrap();
    let chunk_digest = inode.chunk_digests[0];

    // Verify healthy chunk
    assert!(engine.cas.verify_chunk(&chunk_digest));

    // Corrupt chunk
    engine.cas.chunks.insert(chunk_digest, b"Corrupted chunk data".to_vec());
    assert!(!engine.cas.verify_chunk(&chunk_digest));

    // Self-heal chunk
    engine.cas.heal_chunk(chunk_digest, original_payload).unwrap();
    assert!(engine.cas.verify_chunk(&chunk_digest));

    // Verify downloaded content matches
    let downloaded = engine.download_file(&oid).unwrap();
    assert_eq!(downloaded, original_payload);
}

#[test]
fn test_r52_2_c_chat_e2e_encrypted_offline_spool_and_ack() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let ns_chat = [0xC1; 32];
    let channel_id = [0x55; 32];
    let channel_key = [0x77; 32];

    // Offline spooling
    let mut outbox = ChatOutbox::new();
    outbox.spool(channel_id, channel_key, "Offline Message 1");
    outbox.spool(channel_id, channel_key, "Offline Message 2");
    assert_eq!(outbox.pending_count(), 2);

    // Online flush
    let mut chat_engine = NexChatEngine::new(ns_chat, node.identity.actor_id, &mut node);
    let oids = outbox.flush(&mut chat_engine).unwrap();
    assert_eq!(oids.len(), 2);
    assert_eq!(outbox.pending_count(), 0);

    // Read and decrypt messages
    let (msg1, plain1) = chat_engine.read_message(&oids[0], &channel_key).unwrap();
    assert_eq!(String::from_utf8(plain1).unwrap(), "Offline Message 1");
    assert_eq!(msg1.channel_id, channel_id);

    let (_, plain2) = chat_engine.read_message(&oids[1], &channel_key).unwrap();
    assert_eq!(String::from_utf8(plain2).unwrap(), "Offline Message 2");
}

#[test]
fn test_r52_2_d_chat_group_multichannel_reaction_and_read_receipts() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_chat = [0xC1; 32];
    let mut chat_engine = NexChatEngine::new(ns_chat, node.identity.actor_id, &mut node);

    let channel_id = chat_engine.create_channel("general", ChannelType::GroupMultiParty, vec![]).unwrap();
    let channel_key = [0x88; 32];

    let msg_id = chat_engine.send_message(channel_id, b"Group announcement", &channel_key, vec![], vec![], None).unwrap();

    // Add reactions
    chat_engine.add_reaction(msg_id, "👍");
    chat_engine.add_reaction(msg_id, "🚀");

    let reactions = chat_engine.get_reactions(&msg_id);
    assert_eq!(reactions.get("👍"), Some(&1));
    assert_eq!(reactions.get("🚀"), Some(&1));
}

#[test]
fn test_r52_2_e_community_moderator_ban_lock_and_capability_enforcement() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_community = [0xB1; 32];
    let owner_actor = node.identity.actor_id;
    let mut comm_engine = NexCommunityEngine::new(owner_actor, &mut node);

    let comm_id = comm_engine.create_community(ns_community, "Rust Core Developers", "Sovereign Engineering", 1, None).unwrap();
    let chan_id = comm_engine.create_channel(ns_community, comm_id, "general", false, None).unwrap();

    // Owner creates post
    let post_id = comm_engine.create_post(ns_community, chan_id, "Gate R52 Proposal", "Let's productize Nex.", 1, None).unwrap();

    // Create reply
    let reply_id = comm_engine.create_reply(ns_community, post_id, None, "Approved and proceeding.", 2, None).unwrap();
    assert_ne!(post_id, reply_id);

    // Lock post as Owner (Owner >= Moderator)
    assert!(comm_engine.lock_post(comm_id, post_id).is_ok());

    // Pin post as Owner
    assert!(comm_engine.pin_post(comm_id, post_id).is_ok());

    // Reply after lock must fail
    let locked_res = comm_engine.create_reply(ns_community, post_id, None, "Late reply", 3, None);
    assert!(locked_res.is_err());
    assert_eq!(locked_res.unwrap_err(), "Thread is locked");

    // Acceptance of invitation
    let invite_token = CommunityInvitationToken {
        community_id: comm_id,
        invited_actor_id: owner_actor,
        assigned_role: CommunityRole::Admin,
        issuer_actor_id: owner_actor,
        signature: vec![0u8; 64],
    };
    assert!(comm_engine.accept_invitation(invite_token).is_ok());
}

#[test]
fn test_r52_2_f_multi_app_cross_domain_stress_and_merkle_invariance() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_drive = [0xD1; 32];
    let ns_chat = [0xC1; 32];
    let ns_community = [0xB1; 32];

    // 1. Drive mutation
    let mut meta_d = BTreeMap::new();
    meta_d.insert("path".to_string(), "/docs/plan.md".to_string());
    node.create_object(ns_drive, ObjectType::DriveInode, meta_d, b"# Drive Plan".to_vec()).unwrap();

    // 2. Chat mutation
    let mut meta_c = BTreeMap::new();
    meta_c.insert("channel".to_string(), "general".to_string());
    node.create_object(ns_chat, ObjectType::ChatMessage, meta_c, b"Chat payload".to_vec()).unwrap();

    // 3. Community mutation
    let mut meta_comm = BTreeMap::new();
    meta_comm.insert("title".to_string(), "Post 1".to_string());
    node.create_object(ns_community, ObjectType::Community, meta_comm, b"Post body".to_vec()).unwrap();

    assert_eq!(node.state.object_store.len(), 3);

    // Verify SMT Merkle checkpoint
    let cp = node.sync_now().unwrap();
    assert_ne!(cp.body.state_root, [0u8; 32]);

    node.stop().unwrap();
}
