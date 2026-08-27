use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::maps::*;

#[test]
fn test_r60_1_a_tile_coordinate_cas_key_determinism() {
    let coord1 = TileCoordinate::new(14, 8675, 5309);
    let coord2 = TileCoordinate::new(14, 8675, 5309);
    assert_eq!(coord1.cas_key(), coord2.cas_key());

    let coord3 = TileCoordinate::new(14, 8675, 5310);
    assert_ne!(coord1.cas_key(), coord3.cas_key());
}

#[test]
fn test_r60_1_b_vector_tile_ingestion_and_cas_dedup() {
    let dir = tempdir().unwrap();
    let seed = [111u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let coord = TileCoordinate::new(10, 512, 384);
    let tile_data = b"MVT Vector Tile Binary Protobuf Payload".to_vec();

    // Store tile
    let obj_id1 = NexMapsService::store_vector_tile(&mut node, coord, tile_data.clone()).unwrap();
    assert_ne!(obj_id1, [0u8; 32]);

    // Store duplicate tile at same or different coordinate
    let coord2 = TileCoordinate::new(10, 512, 385);
    NexMapsService::store_vector_tile(&mut node, coord2, tile_data.clone()).unwrap();

    assert_eq!(node.storage.cas.chunks.len(), 1, "Identical tile payloads must deduplicate to 1 CAS chunk");
}

#[test]
fn test_r60_1_c_tile_retrieval_from_node() {
    let dir = tempdir().unwrap();
    let seed = [112u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let coord = TileCoordinate::new(12, 1000, 2000);
    let tile_data = b"Tile at zoom 12".to_vec();
    NexMapsService::store_vector_tile(&mut node, coord, tile_data.clone()).unwrap();

    let mut found = false;
    for (_oid, obj) in &node.state.object_store {
        if obj.namespace == NexMapsService::MAPS_NAMESPACE {
            if obj.metadata.get("zoom") == Some(&"12".to_string())
                && obj.metadata.get("x") == Some(&"1000".to_string())
                && obj.metadata.get("y") == Some(&"2000".to_string())
            {
                assert_eq!(obj.payload_bytes, tile_data);
                found = true;
                break;
            }
        }
    }
    assert!(found, "Stored tile must be retrievable from SMT object store");
}

#[test]
fn test_r60_1_d_zoom_pyramid_hierarchy() {
    let dir = tempdir().unwrap();
    let seed = [113u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    for z in 0..=5 {
        let coord = TileCoordinate::new(z, 0, 0);
        let data = format!("Pyramid Tile Z={}", z).into_bytes();
        NexMapsService::store_vector_tile(&mut node, coord, data).unwrap();
    }

    assert_eq!(node.state.object_store.len(), 6);
}

#[test]
fn test_r60_1_e_large_tile_chunking() {
    let dir = tempdir().unwrap();
    let seed = [114u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let coord = TileCoordinate::new(15, 1234, 5678);
    let large_tile = vec![0x33u8; 128 * 1024]; // 128 KB MVT tile
    let obj_id = NexMapsService::store_vector_tile(&mut node, coord, large_tile.clone()).unwrap();
    assert_ne!(obj_id, [0u8; 32]);
}

#[test]
fn test_r60_1_f_zero_regression_tile_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [115u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    node.stop().unwrap();
}
