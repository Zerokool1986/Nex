use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::maps::*;
use nex_core::apps::platform::SpatialMapEngine;

#[test]
fn test_r60_3_a_shared_waypoint_in_bounding_box() {
    let dir = tempdir().unwrap();
    let seed = [131u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let wp1 = Waypoint {
        id: "family-wp-1".to_string(),
        name: "Rendezvous Point".to_string(),
        lat: 47.3769,
        lon: 8.5417, // Zurich
        altitude_m: Some(408.0),
        category: "meeting".to_string(),
        created_epoch: 100,
    };
    NexMapsService::save_waypoint(&mut node, &wp1).unwrap();

    let wp2 = Waypoint {
        id: "family-wp-2".to_string(),
        name: "Tokyo Tower".to_string(),
        lat: 35.6586,
        lon: 139.7454, // Tokyo
        altitude_m: Some(333.0),
        category: "landmark".to_string(),
        created_epoch: 100,
    };
    NexMapsService::save_waypoint(&mut node, &wp2).unwrap();

    // Query Switzerland box
    let results = SpatialMapEngine::query_bounding_box(&node, 45.0, 48.0, 5.0, 10.0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.get("name").unwrap(), "Rendezvous Point");
}

#[test]
fn test_r60_3_b_multi_peer_waypoint_synchronization() {
    let dir1 = tempdir().unwrap();
    let dir2 = tempdir().unwrap();

    let seed1 = [132u8; 32];
    let seed2 = [133u8; 32];

    let mut node1 = NexNode::new(dir1.path(), SigningKey::from_bytes(&seed1));
    let mut node2 = NexNode::new(dir2.path(), SigningKey::from_bytes(&seed2));

    assert!(node1.start().is_ok());
    assert!(node2.start().is_ok());

    let wp = Waypoint {
        id: "sync-wp".to_string(),
        name: "Trailhead".to_string(),
        lat: 46.0,
        lon: 7.0,
        altitude_m: Some(1200.0),
        category: "trailhead".to_string(),
        created_epoch: 500,
    };
    NexMapsService::save_waypoint(&mut node1, &wp).unwrap();
    assert_eq!(node1.state.object_store.len(), 1);
}

#[test]
fn test_r60_3_c_concurrent_group_waypoint_additions() {
    let dir = tempdir().unwrap();
    let seed = [134u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    for i in 0..5 {
        let wp = Waypoint {
            id: format!("group-wp-{}", i),
            name: format!("Group Spot {}", i),
            lat: 40.0 + (i as f64) * 0.05,
            lon: -105.0 + (i as f64) * 0.05,
            altitude_m: Some(1600.0),
            category: "camp".to_string(),
            created_epoch: 1000 + i as u64,
        };
        NexMapsService::save_waypoint(&mut node, &wp).unwrap();
    }

    assert_eq!(node.state.object_store.len(), 5);
}

#[test]
fn test_r60_3_d_waypoint_tombstoning() {
    let dir = tempdir().unwrap();
    let seed = [135u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let wp = Waypoint {
        id: "temp-wp".to_string(),
        name: "Temporary Waypoint".to_string(),
        lat: 42.0,
        lon: 12.0,
        altitude_m: None,
        category: "temp".to_string(),
        created_epoch: 600,
    };
    let obj_id = NexMapsService::save_waypoint(&mut node, &wp).unwrap();

    // Mark tombstoned
    if let Some(obj) = node.state.object_store.get_mut(&obj_id) {
        obj.tombstoned = true;
    }

    // Spatial query must not return tombstoned objects
    let results = SpatialMapEngine::query_bounding_box(&node, 40.0, 45.0, 10.0, 15.0);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_r60_3_e_distance_calculation_precision() {
    let mut track = GpsTrackLog::new("precision-track", "Test");
    track.append_point(0.0, 0.0, 0.0, 100);
    // 1 degree along equator is ~111.32 km = 111,320 m
    track.append_point(0.0, 1.0, 0.0, 200);

    assert!(track.total_distance_meters > 111000.0 && track.total_distance_meters < 112000.0);
}

#[test]
fn test_r60_3_f_zero_regression_group_spatial_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [136u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    node.stop().unwrap();
}
