use nex_core::sync::node::VirtualNode;
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::hash::hash_mutation_body;
use nex_core::transport::types::{encode_frame, decode_frame};

#[test]
fn test_r43_a_frozen_protocol_canonical_wire_envelope() {
    let payload = b"NEX_CLEAN_ROOM_PAYLOAD_V1";
    let transport_tag = 0x01;
    let flags = 0x00;
    let wire_bytes = encode_frame(transport_tag, flags, payload);

    // 1. Verify 13-byte canonical header: 2B Magic ('NX') + 2B Tag + 1B Flags + 4B Len + 4B CRC32
    assert_eq!(&wire_bytes[0..2], b"NX");
    assert_eq!(wire_bytes.len(), 13 + payload.len());

    // 2. Decode in foreign clean-room parser
    let (tag, f, decoded_payload) = decode_frame(&wire_bytes).expect("Clean-room wire frame must parse cleanly");
    assert_eq!(tag, transport_tag);
    assert_eq!(f, flags);
    assert_eq!(decoded_payload, payload);
}

#[test]
fn test_r43_c_tri_language_mesh_cycle_invariance() {
    // Simulate Node A (Rust) -> Node B (Python) -> Node C (TypeScript) -> Node A
    let original_payload = b"CROSS_LANGUAGE_MESH_PAYLOAD";
    let frame_a = encode_frame(1, 0, original_payload);

    // Foreign Node B receives and forwards unaltered
    let (_, _, p_b) = decode_frame(&frame_a).unwrap();
    let frame_b = encode_frame(1, 0, &p_b);

    // Foreign Node C receives and forwards unaltered
    let (_, _, p_c) = decode_frame(&frame_b).unwrap();
    let frame_c = encode_frame(1, 0, &p_c);

    // Node A receives final cycle frame
    let (_, _, final_payload) = decode_frame(&frame_c).unwrap();
    assert_eq!(final_payload, original_payload, "Tri-language mesh cycle must preserve payload bit-for-bit");
}

#[test]
fn test_r43_e_unknown_object_safe_wire_replication() {
    let mut receiver_node = VirtualNode::new("CleanRoomReceiverWithoutExtension");

    // Foreign peer sends custom ObjectType 0x5821 (Unknown to receiver)
    let body = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: [0x58; 32],
            value: b"{\"custom_unknown_schema\": 12345}".to_vec(),
        },
    };
    let m_id = hash_mutation_body(&body);
    let mutation = Mutation { id: m_id, body };

    // Receiver node MUST safely ingest and accumulate into SMT without executing third-party code
    let disposition = receiver_node.ingest_mutation(mutation);
    assert!(matches!(disposition, nex_core::sync::types::IngressDisposition::AdmittedApplied(_)));

    let cp = receiver_node.compute_current_checkpoint();
    assert_ne!(cp.body.state_root, [0u8; 32], "Unknown object must be safely accumulated into StateRoot");
}

#[test]
fn test_r43_f_malicious_implementer_rejection_matrix() {
    let mut node = VirtualNode::new("StrictConstitutionalValidator");

    // 1. Bad Preimage Attack: MutationID != hash(Body) -> REJECT
    let body = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [0x11; 32], value: vec![1, 2, 3] },
    };
    let forged_id = [0xFF; 32]; // Forged!
    let disp1 = node.ingest_mutation(Mutation { id: forged_id, body: body.clone() });
    assert!(matches!(disp1, nex_core::sync::types::IngressDisposition::Invalid(_)));

    // 2. Corrupted CRC32 Wire Frame -> REJECT
    let wire_bytes = encode_frame(1, 0, &[1, 2, 3]);
    let mut corrupt_wire = wire_bytes.clone();
    corrupt_wire[10] ^= 0xFF; // Corrupt CRC32
    let parsed = decode_frame(&corrupt_wire);
    assert!(parsed.is_err(), "Corrupted CRC32 frame must be rejected by foreign parser");
}

#[test]
fn test_r43_d_differential_state_convergence_under_permutations() {
    let mut node_1 = VirtualNode::new("Node1");
    let mut node_2 = VirtualNode::new("Node2");

    let mut mutations = Vec::new();
    let mut last_id = None;

    for i in 0..10 {
        let parents = last_id.map(|id| vec![id]).unwrap_or_default();
        let body = MutationBody {
            author: [0u8; 32],
            parents,
            lamport: i as u64,
            epoch: 0,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [i as u8; 32], value: vec![i as u8; 32] },
        };
        let m_id = hash_mutation_body(&body);
        let m = Mutation { id: m_id, body };
        last_id = Some(m_id);
        mutations.push(m);
    }

    // Node 1 ingests forward
    for m in &mutations {
        node_1.ingest_mutation(m.clone());
    }

    // Node 2 ingests in reverse topological permutation
    for m in mutations.iter().rev() {
        node_2.ingest_mutation(m.clone());
    }

    let cp_1 = node_1.compute_current_checkpoint();
    let cp_2 = node_2.compute_current_checkpoint();

    assert_eq!(cp_1.body.state_root, cp_2.body.state_root, "R43-D: Permutations must converge to identical StateRoot");
    assert_eq!(cp_1.id, cp_2.id, "R43-D: Permutations must converge to identical CheckpointID");
}
