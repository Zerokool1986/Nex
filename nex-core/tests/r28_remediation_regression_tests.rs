use std::collections::HashSet;
use nex_core::sync::node::VirtualNode;
use nex_core::transport::fragmentation::{FragmentationReassembler, MAX_IN_FLIGHT_REASSEMBLIES};
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::hash::hash_mutation_body;

#[test]
fn test_r28_f03_deep_1000_dependency_chain_iterative_releasing() {
    let mut node = VirtualNode::new("IterativeNode");

    // 1. Generate a 1,000-deep linear DAG chain: M0 -> M1 -> M2 ... -> M999
    let mut chain = Vec::with_capacity(1000);

    let b0 = MutationBody {
        parents: vec![],
        lamport: 0,
        epoch: 0,
        author: [0u8; 32],
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [0u8; 32], value: vec![0] },
    };
    let m0 = Mutation { id: hash_mutation_body(&b0), body: b0 };
    chain.push(m0);

    for i in 1..1000u64 {
        let prev_id = chain[(i - 1) as usize].id;
        let mut obj_id = [0u8; 32];
        obj_id[0..8].copy_from_slice(&i.to_le_bytes());

        let b = MutationBody {
            parents: vec![prev_id],
            lamport: i,
            epoch: 0,
            author: [0u8; 32],
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: obj_id, value: i.to_le_bytes().to_vec() },
        };
        let m = Mutation { id: hash_mutation_body(&b), body: b };
        chain.push(m);
    }

    // 2. Ingest M999 down to M1 in reverse order (fills dependency buffer with 999 orphans)
    for m in chain.iter().skip(1).rev() {
        node.ingest_mutation(m.clone());
    }

    assert_eq!(node.dependency_buffer.len(), 999, "All 999 child mutations must wait in dependency buffer");

    // 3. Deliver genesis M0 -> Triggers a 999-cascade unblocking
    // With iterative work-queue, this executes in O(1) stack frames without stack overflow!
    node.ingest_mutation(chain[0].clone());

    assert_eq!(node.dependency_buffer.len(), 0, "All 999 mutations must be cleanly unblocked and admitted");
    assert_eq!(node.dag.len(), 1000, "Full 1,000-node DAG must be completely admitted");

    let cp = node.compute_current_checkpoint();
    assert_eq!(cp.body.boundary.max_lamport, 999);
}

#[test]
fn test_r28_f02_reassembly_stream_clamping_and_ttl_reaping() {
    let mut reassembler = FragmentationReassembler::new();

    // Fill reassembly buffer up to MAX_IN_FLIGHT_REASSEMBLIES (128)
    for i in 0..MAX_IN_FLIGHT_REASSEMBLIES {
        let mut raw_chunk = vec![0u8; 40];
        raw_chunk[0..4].copy_from_slice(&(i as u32).to_le_bytes()); // msg_id
        raw_chunk[32..34].copy_from_slice(&0u16.to_be_bytes());     // chunk_index 0
        raw_chunk[34..36].copy_from_slice(&2u16.to_be_bytes());     // total_chunks 2
        raw_chunk[36..40].copy_from_slice(b"data");

        reassembler.ingest_chunk_with_epoch(&raw_chunk, 10).unwrap();
    }

    assert_eq!(reassembler.in_flight.len(), MAX_IN_FLIGHT_REASSEMBLIES);

    // Advance epoch by 35 seconds (beyond 30s TTL) and prune
    let pruned = reassembler.prune_stale_streams(46, 30);
    assert_eq!(pruned, MAX_IN_FLIGHT_REASSEMBLIES, "All expired in-flight streams must be reclaimed");
    assert_eq!(reassembler.in_flight.len(), 0);
}
