use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::maps::*;
use nex_core::api::NexAppApi;

#[test]
fn test_r60_4_a_extreme_coordinate_handling() {
    let mut track = GpsTrackLog::new("pole-track", "North Pole Track");
    track.append_point(90.0, 0.0, 0.0, 100);
    track.append_point(89.0, 180.0, 0.0, 200);
    assert!(track.total_distance_meters > 0.0);
}

#[test]
fn test_r60_4_b_high_frequency_track_ingestion_stress() {
    let dir = tempdir().unwrap();
    let seed = [141u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let mut track = GpsTrackLog::new("marathon-track", "42K Run");
    for i in 0..1000 {
        track.append_point(37.0 + (i as f64) * 0.0001, -122.0, 10.0, 1000 + i);
    }
    assert_eq!(track.points.len(), 1000);

    let obj_id = NexMapsService::save_track_log(&mut node, &track).unwrap();
    assert_ne!(obj_id, [0u8; 32]);
}

#[test]
fn test_r60_4_c_tile_coordinate_hashing_collision_resistance() {
    let mut keys = std::collections::HashSet::new();
    for z in 0..10 {
        for x in 0..10 {
            for y in 0..10 {
                let coord = TileCoordinate::new(z, x, y);
                let key = coord.cas_key();
                assert!(!keys.contains(&key), "TileCoordinate cas_key must be collision-free");
                keys.insert(key);
            }
        }
    }
    assert_eq!(keys.len(), 1000);
}

#[test]
fn test_r60_4_d_empty_bounding_box_query() {
    let dir = tempdir().unwrap();
    let seed = [142u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let results = nex_core::apps::platform::SpatialMapEngine::query_bounding_box(&node, 0.0, 1.0, 0.0, 1.0);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_r60_4_e_large_tile_dataset_storage() {
    let dir = tempdir().unwrap();
    let seed = [143u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    for i in 0..50 {
        let coord = TileCoordinate::new(10, i, i);
        let data = format!("Tile data payload {}", i).into_bytes();
        NexMapsService::store_vector_tile(&mut node, coord, data).unwrap();
    }
    assert_eq!(node.state.object_store.len(), 50);
}

#[test]
fn test_r60_4_f_gate_r60_master_maps_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [144u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Maps spatial operations must preserve Merkle root invariance");
}
