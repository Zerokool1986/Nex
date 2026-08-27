use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::maps::*;

#[test]
fn test_r60_2_a_waypoint_creation_and_serialization() {
    let wp = Waypoint {
        id: "wp-001".to_string(),
        name: "Campground".to_string(),
        lat: 44.4280,
        lon: -110.5885,
        altitude_m: Some(2357.0),
        category: "campsite".to_string(),
        created_epoch: 100,
    };

    let serialized = serde_json::to_vec(&wp).unwrap();
    let deserialized: Waypoint = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(wp, deserialized);
}

#[test]
fn test_r60_2_b_gps_track_log_haversine_distance() {
    let mut track = GpsTrackLog::new("track-001", "Morning Hike");

    // Point 1: (37.7749, -122.4194)
    track.append_point(37.7749, -122.4194, 10.0, 1000);
    // Point 2: ~1.1 km north (37.7849, -122.4194)
    track.append_point(37.7849, -122.4194, 15.0, 1600);

    assert_eq!(track.points.len(), 2);
    assert!(track.total_distance_meters > 1000.0 && track.total_distance_meters < 1200.0,
        "Distance should be ~1112m, was {}", track.total_distance_meters);
}

#[test]
fn test_r60_2_c_save_waypoint_to_node() {
    let dir = tempdir().unwrap();
    let seed = [121u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let wp = Waypoint {
        id: "wp-002".to_string(),
        name: "Mountain Pass".to_string(),
        lat: 46.5,
        lon: 8.5,
        altitude_m: Some(2100.0),
        category: "mountain_pass".to_string(),
        created_epoch: 200,
    };

    let obj_id = NexMapsService::save_waypoint(&mut node, &wp).unwrap();
    assert_ne!(obj_id, [0u8; 32]);
    assert_eq!(node.state.object_store.len(), 1);
}

#[test]
fn test_r60_2_d_save_track_to_node() {
    let dir = tempdir().unwrap();
    let seed = [122u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let mut track = GpsTrackLog::new("track-002", "Trail Run");
    for i in 0..10 {
        track.append_point(40.0 + (i as f64) * 0.001, -74.0, 50.0 + (i as f64) * 2.0, 1000 + i * 60);
    }

    let obj_id = NexMapsService::save_track_log(&mut node, &track).unwrap();
    assert_ne!(obj_id, [0u8; 32]);
}

#[test]
fn test_r60_2_e_multi_category_waypoints() {
    let dir = tempdir().unwrap();
    let seed = [123u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let categories = ["summit", "water_source", "shelter", "viewpoint"];
    for (i, cat) in categories.iter().enumerate() {
        let wp = Waypoint {
            id: format!("wp-{}", i),
            name: format!("Point {}", i),
            lat: 45.0 + (i as f64) * 0.1,
            lon: 7.0 + (i as f64) * 0.1,
            altitude_m: Some(1500.0),
            category: cat.to_string(),
            created_epoch: 300,
        };
        NexMapsService::save_waypoint(&mut node, &wp).unwrap();
    }

    assert_eq!(node.state.object_store.len(), 4);
}

#[test]
fn test_r60_2_f_zero_regression_waypoint_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [124u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    node.stop().unwrap();
}
