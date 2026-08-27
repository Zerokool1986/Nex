use std::fs;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use nex_core::transport::types::{encode_frame, decode_frame, TransportPacket};
use nex_core::transport::fragmentation::{fragment_payload, FragmentationReassembler};
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::types::KeyType;
use nex_core::sync::types::SyncMessage;
use nex_core::sync::node::VirtualNode;
use nex_core::runtime::node::SovereignNodeRuntime;
use nex_core::storage::wal::WriteAheadLog;
use nex_core::model::{Mutation, MutationBody, CrdtPayload, Checkpoint};
use nex_core::hash::hash_mutation_body;

#[test]
fn test_r30_a_canonical_wire_framing_and_crc_integrity() {
    let raw_payload = b"NEX_PROTOCOL_CANONICAL_TEST_PAYLOAD_BYTES";
    let transport_tag = 0x02; // QUIC
    let flags = 0x00;

    // 1. Encode into 13-byte canonical frame
    let frame = encode_frame(transport_tag, flags, raw_payload);
    assert_eq!(&frame[0..2], b"NX", "Magic must be NX (0x4E58)");
    assert_eq!(&frame[2..4], &transport_tag.to_be_bytes());
    assert_eq!(frame[4], flags);
    assert_eq!(frame.len(), 13 + raw_payload.len());

    // 2. Decode valid frame
    let (decoded_tag, decoded_flags, decoded_payload) = decode_frame(&frame).unwrap();
    assert_eq!(decoded_tag, transport_tag);
    assert_eq!(decoded_flags, flags);
    assert_eq!(decoded_payload, raw_payload);

    // 3. Corrupt 1 byte in payload -> CRC32 failure
    let mut corrupt_frame = frame.clone();
    corrupt_frame[15] ^= 0x55;
    assert!(decode_frame(&corrupt_frame).is_err(), "Corrupted frame payload must fail CRC32 verification");
}

#[test]
fn test_r30_b_foreign_node_chunking_and_reassembly_interop() {
    let mut reassembler = FragmentationReassembler::new();
    let large_payload = vec![0xAB; 2500]; // 2.5KB payload
    let message_id = [0x77; 32];
    let mtu = 600; // Small MTU -> 5 chunks

    // 1. Fragment payload into chunks
    let chunks = fragment_payload(message_id, &large_payload, mtu).unwrap();
    assert_eq!(chunks.len(), 5);

    // 2. Ingest chunks out of order into reassembler (4 -> 2 -> 0 -> 3 -> 1)
    let reorder_indices = [4, 2, 0, 3, 1];
    let mut final_payload = None;

    for &idx in &reorder_indices {
        let res = reassembler.ingest_chunk(&chunks[idx]).unwrap();
        if let Some(complete) = res {
            final_payload = Some(complete);
        }
    }

    assert_eq!(final_payload, Some(large_payload), "R30-B: Out-of-order chunking must cleanly reconstruct original payload");
}

#[test]
fn test_r30_c_cross_node_state_convergence_over_raw_wire_bytes() {
    let mut csprng = OsRng;
    let alice_signing_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_signing_key.verifying_key().to_bytes();
    let alice = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let bob_signing_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_signing_key.verifying_key().to_bytes();
    let bob = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    let namespace = [0x99; 32];
    let image_id = [0x42; 32];

    // Reference Node A (Alice)
    let mut node_a = SovereignNodeRuntime::new(alice, namespace, image_id);
    // Independent Test Node B (Bob)
    let mut node_b = SovereignNodeRuntime::new(bob, namespace, image_id);

    // 1. Alice creates Drive files and Chat messages locally
    let m1 = node_a.drive.create_file("/docs/contract.pdf", [0x11; 32], 4096, "application/pdf", 1);
    let m2 = node_a.drive.create_file("/docs/annex.txt", [0x22; 32], 1024, "text/plain", 2);
    let (_, m3) = node_a.chat.send_message([0xCC; 32], alice, b"wire_protocol_ready".to_vec(), 3);

    node_a.submit_local_mutation(m1.clone());
    node_a.submit_local_mutation(m2.clone());
    node_a.submit_local_mutation(m3.clone());

    let cp_a = node_a.checkpoint();

    // 2. Alice encodes mutations into raw wire bytes
    let mutations = vec![m1, m2, m3];
    let mut wire_byte_pipe = Vec::new();

    for m in mutations {
        let sync_envelope = SyncMessage::DirectMutationBroadcast(m);
        let serialized = serde_json::to_vec(&sync_envelope).unwrap();
        let frame = encode_frame(0x02, 0x00, &serialized);
        wire_byte_pipe.push(frame);
    }

    // 3. Bob receives raw wire frames across transport
    for frame_bytes in wire_byte_pipe {
        let (_, _, payload) = decode_frame(&frame_bytes).unwrap();
        let sync_msg: SyncMessage = serde_json::from_slice(&payload).unwrap();
        if let SyncMessage::DirectMutationBroadcast(m) = sync_msg {
            node_b.submit_local_mutation(m);
        }
    }

    let cp_b = node_b.checkpoint();

    // 4. Assert 100% byte-for-byte state convergence
    assert_eq!(cp_a.id, cp_b.id, "R30-C: Independent nodes must converge to identical CheckpointID over wire bytes");
    assert_eq!(cp_a.body.state_root, cp_b.body.state_root);
    assert_eq!(cp_a.body.causal_root, cp_b.body.causal_root);
    assert_eq!(cp_a.body.admission_root, cp_b.body.admission_root);
}

#[test]
fn test_r30_d_independent_proof_statement_verification_boundary() {
    // Demonstrates independent proof verification over pure serialized statement bytes
    let mut node = VirtualNode::new("ProverNode");
    let b = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [0x55; 32], value: b"zk_payload".to_vec() },
    };
    let m = Mutation { id: hash_mutation_body(&b), body: b };
    node.ingest_mutation(m);

    let checkpoint = node.compute_current_checkpoint();

    // Serialize checkpoint into standalone binary statement
    let serialized_statement = serde_json::to_vec(&checkpoint).unwrap();

    // Independent Foreign Verifier (No reference node instance or private memory state)
    let deserialized: Checkpoint = serde_json::from_slice(&serialized_statement).unwrap();
    let recomputed_id = nex_core::hash::hash_checkpoint_body(&deserialized.body);

    assert_eq!(deserialized.id, recomputed_id, "R30-D: Independent verifier must validate checkpoint preimage equality");
    assert_eq!(deserialized.id, checkpoint.id);
}

#[test]
fn test_r30_e_interoperable_wal_export_and_recovery() {
    let temp_dir = std::env::temp_dir().join(format!("nex_r30_wal_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&temp_dir).unwrap();
    let wal_path = temp_dir.join("node_a_export.wal");

    // Node A writes 20 mutations to WAL
    let mut wal_a = WriteAheadLog::open(&wal_path).unwrap();
    let mut node_a = VirtualNode::new("NodeA");

    let mut prev = None;
    for i in 0..20u64 {
        let parents = prev.map(|id| vec![id]).unwrap_or_default();
        let b = MutationBody {
            author: [0u8; 32],
            parents,
            lamport: i,
            epoch: 0,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [i as u8; 32], value: vec![i as u8] },
        };
        let m = Mutation { id: hash_mutation_body(&b), body: b };
        prev = Some(m.id);

        wal_a.append_mutation(&m).unwrap();
        node_a.ingest_mutation(m);
    }
    drop(wal_a);

    let cp_a = node_a.compute_current_checkpoint();

    // Node B (Independent Test Node) imports WAL from disk
    let imported_mutations = WriteAheadLog::recover(&wal_path).unwrap();
    assert_eq!(imported_mutations.len(), 20);

    let mut node_b = VirtualNode::new("NodeB");
    for m in imported_mutations {
        node_b.ingest_mutation(m);
    }

    let cp_b = node_b.compute_current_checkpoint();

    assert_eq!(cp_a.id, cp_b.id, "R30-E: Imported WAL must produce identical canonical CheckpointID");
    assert_eq!(cp_a.body.state_root, cp_b.body.state_root);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_r30_f_multi_transport_wire_payload_neutrality() {
    let raw_msg = b"NEX_MULTI_TRANSPORT_NEUTRAL_PAYLOAD";

    // Encoded over Mesh (tag 0x01)
    let frame_mesh = encode_frame(0x01, 0x00, raw_msg);
    // Encoded over QUIC (tag 0x02)
    let frame_quic = encode_frame(0x02, 0x00, raw_msg);
    // Encoded over WebRTC (tag 0x03)
    let frame_webrtc = encode_frame(0x03, 0x00, raw_msg);

    // Decode all frames
    let (_, _, payload_mesh) = decode_frame(&frame_mesh).unwrap();
    let (_, _, payload_quic) = decode_frame(&frame_quic).unwrap();
    let (_, _, payload_webrtc) = decode_frame(&frame_webrtc).unwrap();

    // Assert wire payload semantics are 100% identical regardless of carrier tag
    assert_eq!(payload_mesh, raw_msg);
    assert_eq!(payload_quic, raw_msg);
    assert_eq!(payload_webrtc, raw_msg);
}

#[test]
fn test_r30_g_golden_vectors_conformance_suite() {
    let json_str = include_str!("golden_vectors_r30.json");
    let v: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // 1. Identity Vectors
    for item in v["identity_vectors"].as_array().unwrap() {
        let pk_hex = item["public_key_hex"].as_str().unwrap();
        let pk = hex::decode(pk_hex).unwrap();
        let actor = derive_actor_id(KeyType::Ed25519, &pk);
        assert_eq!(hex::encode(actor), item["expected_actor_id_hex"].as_str().unwrap());
    }

    // 2. Wire Framing Vectors
    for item in v["wire_frame_vectors"].as_array().unwrap() {
        let payload_hex = item["payload_hex"].as_str().unwrap();
        let payload = hex::decode(payload_hex).unwrap();
        let tag = item["transport_tag"].as_u64().unwrap() as u16;
        let flags = item["flags"].as_u64().unwrap() as u8;

        let frame = encode_frame(tag, flags, &payload);
        assert_eq!(hex::encode(&frame), item["expected_frame_hex"].as_str().unwrap());

        let (dec_tag, dec_flags, dec_payload) = decode_frame(&frame).unwrap();
        assert_eq!(dec_tag, tag);
        assert_eq!(dec_flags, flags);
        assert_eq!(dec_payload, payload);
    }

    // 3. SMT Key Vectors
    for item in v["smt_key_vectors"].as_array().unwrap() {
        let m_id_hex = item["mutation_id_hex"].as_str().unwrap();
        let m_id_bytes = hex::decode(m_id_hex).unwrap();
        let mut m_id = [0u8; 32];
        m_id.copy_from_slice(&m_id_bytes);
        let smt_key = nex_core::accumulator::sha256_smt_key(&m_id);
        assert_eq!(hex::encode(smt_key), item["expected_smt_key_hex"].as_str().unwrap());
    }
}

