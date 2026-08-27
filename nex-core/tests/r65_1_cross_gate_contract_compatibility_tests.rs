use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use tempfile::tempdir;
use sha2::{Sha256, Digest};
use nex_core::runtime::node::NexNode;
use nex_core::object::types::{NamespaceID, ObjectType};
use nex_core::apps::groups::*;
use nex_core::apps::maps::*;
use nex_core::apps::resources::*;
use nex_core::apps::compute::*;
use nex_core::apps::discovery::*;
use nex_core::api::NexAppApi;

#[test]
fn test_r65_1_a_cross_gate_object_serialization_roundtrip() {
    let dir = tempdir().unwrap();
    let seed = [101u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Sovereign Alpha", admin);
    let member = [0x02u8; 32];
    group.add_member(member, GroupRole::Member);

    // Save group to node
    let obj_id = NexGroupsService::save_group_state(&mut node, &group).unwrap();
    assert_ne!(obj_id, [0u8; 32]);

    // Retrieve and verify
    let obj = node.read_object(&obj_id).unwrap();
    let recovered_group: GroupState = serde_json::from_slice(&obj.payload_bytes).unwrap();
    assert_eq!(recovered_group.name, "Sovereign Alpha");
    assert_eq!(recovered_group.members.len(), 2);
    assert!(recovered_group.is_active_member(&member));
}

#[test]
fn test_r65_1_b_maps_tile_and_waypoint_cross_compatibility() {
    let dir = tempdir().unwrap();
    let seed = [102u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let coord = TileCoordinate::new(12, 2154, 1432);
    let tile_data = vec![0xDE, 0xAD, 0xBE, 0xEF];

    let tile_id = NexMapsService::store_vector_tile(&mut node, coord, tile_data.clone()).unwrap();
    assert_ne!(tile_id, [0u8; 32]);

    let wp = Waypoint {
        id: "wp_001".to_string(),
        name: "Checkpoint Alpha".to_string(),
        lat: 37.7749,
        lon: -122.4194,
        altitude_m: Some(15.0),
        category: "Security".to_string(),
        created_epoch: 100,
    };
    let wp_id = NexMapsService::save_waypoint(&mut node, &wp).unwrap();
    assert_ne!(wp_id, tile_id);
}

#[test]
fn test_r65_1_c_erasure_shards_smt_roundtrip() {
    let dir = tempdir().unwrap();
    let seed = [103u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let payload = b"Cross-gate system integration payload testing.".to_vec();
    let shards = ErasureCoder::split(&payload, 4);

    let ns: NamespaceID = [0xEE; 32];
    let mut shard_obj_ids = Vec::new();

    for shard in &shards {
        let mut meta = BTreeMap::new();
        meta.insert("shard_index".to_string(), shard.shard_index.to_string());
        let serialized = serde_json::to_vec(shard).unwrap();
        let obj_id = node.create_object(ns, ObjectType::Synthetic(20), meta, serialized).unwrap();
        shard_obj_ids.push(obj_id);
    }

    assert_eq!(shard_obj_ids.len(), 5);

    // Reconstruct payload directly from stored shards
    let mut recovered_shards = Vec::new();
    for id in &shard_obj_ids[0..4] {
        let obj = node.read_object(id).unwrap();
        let s: ShardDescriptor = serde_json::from_slice(&obj.payload_bytes).unwrap();
        recovered_shards.push(s);
    }

    let reconstructed = ErasureCoder::reconstruct(&recovered_shards, 4, payload.len()).unwrap();
    assert_eq!(reconstructed, payload);
}

#[test]
fn test_r65_1_d_compute_job_descriptor_serialization() {
    let bytecode = vec![0x01, 0x02];
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x55u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![[0x01u8; 32], [0x02u8; 32]],
        fuel_limit: 5000,
        memory_limit_bytes: 64 * 1024,
    };

    let serialized = serde_json::to_vec(&job).unwrap();
    let deserialized: ComputeJobDescriptor = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(job, deserialized);
}

#[test]
fn test_r65_1_e_dht_locator_serialization() {
    let locator = PeerLocator {
        actor_id: [0xAAu8; 32],
        socket_addr: "192.168.1.100:9000".to_string(),
        last_seen_epoch: 42,
    };

    let serialized = serde_json::to_vec(&locator).unwrap();
    let deserialized: PeerLocator = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(locator, deserialized);
}

#[test]
fn test_r65_1_f_zero_regression_cross_gate_contracts() {
    let dir = tempdir().unwrap();
    let seed = [104u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root);
}
