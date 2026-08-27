use nex_core::apps::drive::{DriveEngine, derive_drive_object_id};
use nex_core::apps::chat::ChatEngine;
use nex_core::apps::community::{CommunityEngine, CommunityRole};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::identity::types::{KeyType, CapabilityToken, CapabilityProof, OP_ALL, OP_REGISTER_LWW, OP_OBJECT_TOMBSTONE};
use nex_core::sync::node::VirtualNode;
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::hash::hash_mutation_body;

#[test]
fn test_r26_a_nex_drive_file_tree_operations() {
    let namespace_id = [0xAA; 32];
    let mut drive = DriveEngine::new(namespace_id);

    // 1. Create file /docs/spec.pdf
    let m1 = drive.create_file("/docs/spec.pdf", [0x11; 32], 1024, "application/pdf", 1);
    assert_eq!(drive.files.len(), 1);
    assert_eq!(drive.files["/docs/spec.pdf"].content_hash, [0x11; 32]);

    // 2. Ingest into VirtualNode DAG
    let mut node = VirtualNode::new("DriveNode");
    node.ingest_mutation(m1);

    // 3. Delete file
    let m2 = drive.delete_file("/docs/spec.pdf", 2);
    assert_eq!(drive.files.len(), 0);
    node.ingest_mutation(m2);

    let cp = node.compute_current_checkpoint();
    assert_ne!(cp.body.state_root, [0u8; 32]);
}

#[test]
fn test_r26_c_nex_chat_messaging_and_read_receipts() {
    let mut chat = ChatEngine::new();
    let channel_id = [0xCC; 32];
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);

    // 1. Alice sends message 1
    let (msg1, m1) = chat.send_message(channel_id, alice, b"encrypted_hello".to_vec(), 1);
    assert_eq!(msg1.sequence_index, 1);

    // 2. Bob sends message 2
    let (msg2, m2) = chat.send_message(channel_id, bob, b"encrypted_world".to_vec(), 2);
    assert_eq!(msg2.sequence_index, 2);

    // 3. Alice marks read up to message 2
    let m3 = chat.mark_read(channel_id, alice, 2, 3);
    assert_eq!(chat.read_receipts[&(channel_id, alice)], 2);

    let mut node = VirtualNode::new("ChatNode");
    node.ingest_mutation(m1);
    node.ingest_mutation(m2);
    node.ingest_mutation(m3);

    let cp = node.compute_current_checkpoint();
    assert_ne!(cp.id, [0u8; 32]);
}

#[test]
fn test_r26_d_attenuated_subfolder_sharing() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let namespace = [0xDD; 32];

    let shared_subfolder_obj = derive_drive_object_id(&namespace, "/photos/family");

    // Alice grants Bob write access strictly to /photos/family object
    let token = CapabilityToken {
        issuer: alice,
        subject: bob,
        namespace,
        object_id: Some(shared_subfolder_obj),
        allowed_operations: OP_REGISTER_LWW,
        delegation_depth: 1,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let proof = CapabilityProof {
        token,
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x11; 64],
    };

    let empty_rev = std::collections::BTreeMap::new();

    // 1. Bob creates file in shared subfolder -> Authorized
    let auth_ok = nex_core::identity::verifier::verify_capability_chain(
        &proof,
        OP_REGISTER_LWW,
        &namespace,
        Some(&shared_subfolder_obj),
        10,
        &empty_rev,
        &alice,
    );
    assert_eq!(auth_ok, Ok(bob));

    // 2. Bob attempts to modify unshared /finance object -> Rejected
    let finance_obj = derive_drive_object_id(&namespace, "/finance");
    let auth_err = nex_core::identity::verifier::verify_capability_chain(
        &proof,
        OP_REGISTER_LWW,
        &namespace,
        Some(&finance_obj),
        10,
        &empty_rev,
        &alice,
    );
    assert_eq!(auth_err, Err(nex_core::identity::types::AuthorizationError::ObjectMismatch));
}

#[test]
fn test_r26_e_community_role_delegation_and_tombstoning() {
    let owner = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let mod_actor = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let community_id = [0xEE; 32];

    let mut comm = CommunityEngine::new(community_id, owner);
    comm.add_member(mod_actor, CommunityRole::Moderator, 1);

    let msg_obj_id = [0x99; 32];

    // Owner issues moderator capability proof to mod_actor
    let mod_token = CapabilityToken {
        issuer: owner,
        subject: mod_actor,
        namespace: community_id,
        object_id: Some(msg_obj_id),
        allowed_operations: OP_OBJECT_TOMBSTONE,
        delegation_depth: 1,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let mod_proof = CapabilityProof {
        token: mod_token,
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x22; 64],
    };

    // Moderator tombstones abusive message
    let tombstone_res = comm.tombstone_message(msg_obj_id, &mod_proof, 10);
    assert!(tombstone_res.is_ok(), "R26-E: Moderator capability must permit message tombstone");
    assert!(comm.tombstoned_messages.contains_key(&msg_obj_id));
}

#[test]
fn test_r26_f_offline_conflict_reconciliation() {
    let namespace = [0x55; 32];
    let file_obj_id = derive_drive_object_id(&namespace, "/notes.txt");

    // Genesis
    let b0 = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: file_obj_id, value: b"initial_text".to_vec() },
    };
    let m0 = Mutation { id: hash_mutation_body(&b0), body: b0 };

    // Offline replica A edits file (Lamport 1, Epoch 0, "Alice text")
    let b_a = MutationBody {
        author: [0u8; 32],
        parents: vec![m0.id],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: file_obj_id, value: b"Alice_text".to_vec() },
    };
    let m_a = Mutation { id: hash_mutation_body(&b_a), body: b_a };

    // Offline replica B edits file concurrently (Lamport 2, Epoch 0, "Bob text")
    let b_b = MutationBody {
        author: [0u8; 32],
        parents: vec![m0.id],
        lamport: 2,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: file_obj_id, value: b"Bob_text".to_vec() },
    };
    let m_b = Mutation { id: hash_mutation_body(&b_b), body: b_b };

    // Node 1 receives M0 -> M_a -> M_b
    let mut node1 = VirtualNode::new("Node1");
    node1.ingest_mutation(m0.clone());
    node1.ingest_mutation(m_a.clone());
    node1.ingest_mutation(m_b.clone());

    // Node 2 receives M0 -> M_b -> M_a
    let mut node2 = VirtualNode::new("Node2");
    node2.ingest_mutation(m0);
    node2.ingest_mutation(m_b);
    node2.ingest_mutation(m_a);

    let cp1 = node1.compute_current_checkpoint();
    let cp2 = node2.compute_current_checkpoint();

    // Assert both nodes deterministically resolve the conflict identically (Bob's edit wins via higher Lamport)
    assert_eq!(cp1.id, cp2.id, "R26-F: Offline conflict reconciliation must converge to identical CheckpointID");
    assert_eq!(cp1.body.state_root, cp2.body.state_root);

    let (val1, _, _, _) = &node1.crdt_state[&file_obj_id];
    let (val2, _, _, _) = &node2.crdt_state[&file_obj_id];
    assert_eq!(val1.as_ref().unwrap(), b"Bob_text");
    assert_eq!(val2.as_ref().unwrap(), b"Bob_text");
}
