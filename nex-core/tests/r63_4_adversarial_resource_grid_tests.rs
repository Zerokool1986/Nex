use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::resources::*;
use nex_core::api::NexAppApi;

#[test]
fn test_r63_4_a_shard_health_transitions() {
    let mut auditor = ShardHealthAuditor::new();
    let chunk = [0xAAu8; 32];

    let p1 = [0x01u8; 32];
    let p2 = [0x02u8; 32];
    let p3 = [0x03u8; 32];
    let p4 = [0x04u8; 32];

    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Critical);

    auditor.register_provider(chunk, p1);
    auditor.register_provider(chunk, p2);
    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Critical);

    auditor.register_provider(chunk, p3);
    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Degraded);

    auditor.register_provider(chunk, p4);
    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Healthy);

    // Unregister p1
    auditor.unregister_provider(&chunk, &p1);
    assert_eq!(auditor.audit_health(&chunk, 3, 4), ShardHealthStatus::Degraded);
}

#[test]
fn test_r63_4_b_high_throughput_por_batch() {
    for i in 0..1000 {
        let shard = format!("chunk_data_{}", i).into_bytes();
        let proof = ProofOfRetrievability::prove_storage(&shard, i as u64);
        assert!(ProofOfRetrievability::verify_proof(proof, &shard, i as u64));
    }
}

#[test]
fn test_r63_4_c_large_payload_sharding_stress() {
    let big_payload = vec![0x42u8; 100 * 1024]; // 100 KB
    let shards = ErasureCoder::split(&big_payload, 10);
    assert_eq!(shards.len(), 11);

    // Drop shard 5
    let mut available = Vec::new();
    for (i, s) in shards.iter().enumerate() {
        if i != 5 {
            available.push(s.clone());
        }
    }

    let recovered = ErasureCoder::reconstruct(&available, 10, big_payload.len()).unwrap();
    assert_eq!(recovered, big_payload);
}

#[test]
fn test_r63_4_d_shard_id_determinism_and_collision_resistance() {
    let desc1 = ShardDescriptor {
        chunk_hash: [0x11u8; 32],
        shard_index: 0,
        total_shards: 4,
        data: vec![1, 2, 3],
    };
    let desc2 = ShardDescriptor {
        chunk_hash: [0x11u8; 32],
        shard_index: 1,
        total_shards: 4,
        data: vec![1, 2, 3],
    };

    assert_ne!(desc1.shard_id(), desc2.shard_id());
    assert_eq!(desc1.shard_id(), desc1.shard_id());
}

#[test]
fn test_r63_4_e_unknown_chunk_health() {
    let auditor = ShardHealthAuditor::new();
    let unknown_chunk = [0x99u8; 32];
    assert_eq!(auditor.audit_health(&unknown_chunk, 1, 2), ShardHealthStatus::Critical);
}

#[test]
fn test_r63_4_f_gate_r63_master_resource_grid_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [199u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Resource Grid operations must preserve Merkle root invariance");
}
