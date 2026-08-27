use std::collections::BTreeMap;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use nex_core::api::NexCoreRuntime;
use nex_core::apps::drive::{NexDriveEngine, CasChunkStore};
use nex_core::apps::photos::{NexPhotosEngine, MediaMetadata};
use nex_core::apps::chat::{NexChatEngine, ChannelType};
use nex_core::identity::types::{
    KeyType, CapabilityToken, CapabilityProof, OP_READ, OP_WRITE
};
use nex_core::identity::verifier::{derive_actor_id, verify_capability_chain};
use nex_core::sync::node::VirtualNode;
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::hash::hash_mutation_body;

#[test]
fn test_r40_b_hostile_capability_composition_and_escalation_attacks() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let mallory_key = SigningKey::generate(&mut csprng);
    let mallory_pubkey = mallory_key.verifying_key().to_bytes();
    let mallory_actor = derive_actor_id(KeyType::Ed25519, &mallory_pubkey);

    let photo_ns = [0xC1; 32];
    let drive_ns = [0xD1; 32];
    let photo_obj_a = [0x11; 32];
    let photo_obj_b = [0x22; 32];

    // Alice grants Mallory READ access to photo_obj_a
    let token = CapabilityToken {
        issuer: alice_actor,
        subject: mallory_actor,
        allowed_operations: OP_READ,
        namespace: photo_ns,
        object_id: Some(photo_obj_a),
        not_before_epoch: 0,
        expires_at_epoch: 10,
        delegation_depth: 1,
        parent_token_hash: None,
    };
    let sig = alice_key.sign(&token.canonical_bytes());
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(alice_pubkey.to_vec()),
        signature: sig.to_bytes().to_vec(),
        parent_proof: None,
    };

    let empty_revocations = BTreeMap::new();

    // 1. Attack: Use photo_obj_a token to read photo_obj_b -> REJECT
    let res_obj_escape = verify_capability_chain(
        &proof,
        OP_READ,
        &photo_ns,
        Some(&photo_obj_b),
        1,
        &empty_revocations,
        &alice_actor,
    );
    assert!(res_obj_escape.is_err(), "Subtree object escape must be rejected");

    // 2. Attack: Use photo token in Drive namespace -> REJECT
    let res_ns_escape = verify_capability_chain(
        &proof,
        OP_READ,
        &drive_ns,
        None,
        1,
        &empty_revocations,
        &alice_actor,
    );
    assert!(res_ns_escape.is_err(), "Namespace escape must be rejected");

    // 3. Attack: Mallory attempts to sign a child token granting OP_WRITE (Elevation) -> REJECT
    let forged_child_token = CapabilityToken {
        issuer: mallory_actor,
        subject: mallory_actor,
        allowed_operations: OP_WRITE, // Elevated
        namespace: photo_ns,
        object_id: Some(photo_obj_a),
        not_before_epoch: 0,
        expires_at_epoch: 10,
        delegation_depth: 0,
        parent_token_hash: Some(proof.token.hash()),
    };
    let forged_sig = mallory_key.sign(&forged_child_token.canonical_bytes());
    let forged_proof = CapabilityProof {
        token: forged_child_token,
        issuer_pubkey: Some(mallory_pubkey.to_vec()),
        signature: forged_sig.to_bytes().to_vec(),
        parent_proof: Some(Box::new(proof.clone())),
    };
    let res_forged = verify_capability_chain(
        &forged_proof,
        OP_WRITE,
        &photo_ns,
        Some(&photo_obj_a),
        1,
        &empty_revocations,
        &alice_actor,
    );
    assert!(res_forged.is_err(), "Downstream permission elevation must be rejected");
}

#[test]
fn test_r40_c_identity_revocation_race_audit() {
    let mut csprng = OsRng;
    let root_key = SigningKey::generate(&mut csprng);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device_key = SigningKey::generate(&mut csprng);
    let device_pubkey = device_key.verifying_key().to_bytes();
    let device_actor = derive_actor_id(KeyType::Ed25519, &device_pubkey);

    let ns = [0xEE; 32];
    let token = CapabilityToken {
        issuer: root_actor,
        subject: device_actor,
        allowed_operations: OP_READ | OP_WRITE,
        namespace: ns,
        object_id: None,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        delegation_depth: 1,
        parent_token_hash: None,
    };
    let sig = root_key.sign(&token.canonical_bytes());
    let token_hash = token.hash();
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_pubkey.to_vec()),
        signature: sig.to_bytes().to_vec(),
        parent_proof: None,
    };

    let mut revocations = BTreeMap::new();

    // Valid prior to revocation fence
    assert!(verify_capability_chain(&proof, OP_WRITE, &ns, None, 1, &revocations, &root_actor).is_ok());

    // Root publishes active revocation fence at Epoch 5
    revocations.insert(token_hash, 5);

    // Concurrent mutation attempt at Epoch 5 -> REJECTED
    assert!(verify_capability_chain(&proof, OP_WRITE, &ns, None, 5, &revocations, &root_actor).is_err());

    // Replay attempt of stale mutation at Epoch 6 -> REJECTED
    assert!(verify_capability_chain(&proof, OP_WRITE, &ns, None, 6, &revocations, &root_actor).is_err());
}

#[test]
fn test_r40_d_cas_multi_app_reachability_and_race_stability() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let mut cas = CasChunkStore::new();

    // 1. Shared 4MB asset
    let blob = vec![0x9A; 4 * 1024 * 1024];
    let (content_root, chunk_digests) = cas.store_file(&blob);

    let runtime1 = NexCoreRuntime::new(alice_key.clone(), None);
    let runtime2 = NexCoreRuntime::new(alice_key.clone(), None);
    let runtime3 = NexCoreRuntime::new(alice_key, None);

    let mut drive = NexDriveEngine::new([0xD3; 32], runtime1);
    let mut photos = NexPhotosEngine::new([0xC9; 32], alice_actor, runtime2, cas.clone());
    let mut chat = NexChatEngine::new([0xCA; 32], alice_actor, runtime3);

    let drive_file = drive.upload_file("shared_doc.bin", "application/octet-stream", &blob, None).unwrap();

    let meta = MediaMetadata {
        width: 100, height: 100, capture_timestamp: 0,
        camera_make: "X".into(), camera_model: "Y".into(),
        lens_model: None, iso: None, exposure_time: None, f_number: None,
        gps_latitude: None, gps_longitude: None,
    };
    let photo_id = photos.import_photo("shared_img.png", "image/png", &blob, meta).unwrap();

    let chan_id = chat.create_channel("Group", ChannelType::GroupMultiParty, vec![]).unwrap();
    let _chat_msg = chat.send_message(chan_id, b"File attachment", &[0x22; 32], vec![], vec![content_root], None).unwrap();

    // 2. Drive deletes its reference
    drive.delete_file("shared_doc.bin", drive_file, None).unwrap();

    // 3. CAS chunks MUST still exist and remain readable via Photos and Chat
    for digest in &chunk_digests {
        assert!(cas.has_chunk(digest), "Chunk must remain reachable while Photos/Chat reference it");
    }

    // 4. Photos deletes its reference
    photos.delete_photo(photo_id, None).unwrap();
    for digest in &chunk_digests {
        assert!(cas.has_chunk(digest), "Chunk must remain reachable while Chat references it");
    }
}

#[test]
fn test_r40_e_cross_app_dag_state_root_convergence_fuzzing() {
    let mut node_a = VirtualNode::new("NodeA");
    let mut node_b = VirtualNode::new("NodeB");

    // Construct 6 sequential cross-application mutations
    let mut mutations = Vec::new();
    let mut last_id = None;

    for i in 0..6 {
        let parents = last_id.map(|id| vec![id]).unwrap_or_default();
        let payload = match i % 3 {
            0 => CrdtPayload::AddLWW { id: [i as u8; 32], value: vec![0x10 + i as u8] }, // Drive/Photo
            1 => CrdtPayload::AddLWW { id: [0x50 + i as u8; 32], value: vec![0x20 + i as u8] }, // Chat/Comm
            _ => CrdtPayload::Tombstone { id: [0; 32] }, // Deletion
        };
        let body = MutationBody {
            author: [0u8; 32],
            parents,
            lamport: i as u64,
            epoch: 0,
            is_resurrect: false,
            payload,
        };
        let m_id = hash_mutation_body(&body);
        let m = Mutation { id: m_id, body };
        last_id = Some(m_id);
        mutations.push(m);
    }

    // Node A ingests in forward order (0 -> 1 -> 2 -> 3 -> 4 -> 5)
    for m in &mutations {
        node_a.ingest_mutation(m.clone());
    }

    // Node B ingests in out-of-order reverse dependency arrival (5 -> 4 -> 3 -> 2 -> 1 -> 0)
    for m in mutations.iter().rev() {
        node_b.ingest_mutation(m.clone());
    }

    // Both nodes compute checkpoints
    let cp_a = node_a.compute_current_checkpoint();
    let cp_b = node_b.compute_current_checkpoint();

    assert_eq!(cp_a.body.state_root, cp_b.body.state_root, "StateRoot must be byte-for-byte identical");
    assert_eq!(cp_a.body.causal_root, cp_b.body.causal_root, "CausalRoot must be byte-for-byte identical");
    assert_eq!(cp_a.id, cp_b.id, "CheckpointID must be identical");
}

#[test]
fn test_r40_g_tombstone_permanence_and_anti_resurrection() {
    let mut node = VirtualNode::new("TombstoneGuardNode");

    let obj_id = [0x77; 32];

    // 1. Genesis Add mutation
    let m1_body = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: obj_id, value: b"LIVE_DATA".to_vec() },
    };
    let m1_id = hash_mutation_body(&m1_body);
    node.ingest_mutation(Mutation { id: m1_id, body: m1_body });

    // 2. Tombstone mutation
    let m2_body = MutationBody {
        author: [0u8; 32],
        parents: vec![m1_id],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::Tombstone { id: obj_id },
    };
    let m2_id = hash_mutation_body(&m2_body);
    node.ingest_mutation(Mutation { id: m2_id, body: m2_body });

    // Verify object is tombstoned
    assert_eq!(node.crdt_state.get(&obj_id).unwrap().0, None);

    // 3. Delayed old mutation with lower Lamport rank arriving -> Must NOT resurrect object
    let stale_body = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: obj_id, value: b"STALE_RESURRECT_ATTEMPT".to_vec() },
    };
    let stale_id = hash_mutation_body(&stale_body);
    let _ = node.ingest_mutation(Mutation { id: stale_id, body: stale_body });

    // State MUST remain tombstoned
    assert_eq!(node.crdt_state.get(&obj_id).unwrap().0, None, "Tombstone must permanently defeat stale resurrection attempts");
}
