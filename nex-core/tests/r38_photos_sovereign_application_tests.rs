use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::api::NexCoreRuntime;
use nex_core::apps::drive::CasChunkStore;
use nex_core::apps::photos::{NexPhotosEngine, MediaMetadata};
use nex_core::identity::types::KeyType;
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r38_a_photo_import_and_derivative_pipeline() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let cas = CasChunkStore::new();
    let ns = [0xB1; 32];
    let mut engine = NexPhotosEngine::new(ns, alice_actor, runtime, cas);

    // Create 3MB RAW Image Simulation (2 chunks in CAS)
    let raw_image_bytes = vec![0xEE; 3 * 1024 * 1024];

    let meta = MediaMetadata {
        width: 7008,
        height: 4672,
        capture_timestamp: 1724180000,
        camera_make: "Sony".into(),
        camera_model: "ILCE-7M4".into(),
        lens_model: Some("FE 24-70mm F2.8 GM II".into()),
        iso: Some(100),
        exposure_time: Some("1/500".into()),
        f_number: Some(2.8),
        gps_latitude: Some(37.7749),
        gps_longitude: Some(-122.4194),
    };

    let media_id = engine.import_photo("DSC_0042.ARW", "image/x-sony-arw", &raw_image_bytes, meta).unwrap();
    assert!(engine.photos.contains_key(&media_id));

    let photo = engine.photos.get(&media_id).unwrap();
    assert_eq!(photo.raw_byte_size, 3 * 1024 * 1024);
    assert_eq!(photo.raw_chunk_digests.len(), 2);
    assert_ne!(photo.thumbnail_digest, [0u8; 32]);
    assert_ne!(photo.preview_content_root, [0u8; 32]);
}

#[test]
fn test_r38_c_cross_app_cas_deduplication_between_drive_and_photos() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let mut cas = CasChunkStore::new();
    let sample_image = vec![0x7A; 1024 * 1024]; // 1MB image

    // Store in CAS via Drive workflow
    let (drive_root, drive_digests) = cas.store_file(&sample_image);
    let initial_chunk_count = cas.chunks.len();

    // Import into Photos using the exact same CAS store
    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut engine = NexPhotosEngine::new([0xB2; 32], alice_actor, runtime, cas);

    let meta = MediaMetadata {
        width: 1920,
        height: 1080,
        capture_timestamp: 1724180100,
        camera_make: "Apple".into(),
        camera_model: "iPhone 15 Pro".into(),
        lens_model: None,
        iso: None,
        exposure_time: None,
        f_number: None,
        gps_latitude: None,
        gps_longitude: None,
    };

    engine.import_photo("IMG_0001.JPG", "image/jpeg", &sample_image, meta).unwrap();
    let photo = engine.photos.values().next().unwrap();

    // Verify chunk deduplication
    assert_eq!(photo.raw_content_root, drive_root);
    assert_eq!(photo.raw_chunk_digests, drive_digests);
}

#[test]
fn test_r38_e_exif_gps_privacy_redaction() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let cas = CasChunkStore::new();
    let mut engine = NexPhotosEngine::new([0xB3; 32], alice_actor, runtime, cas);

    let meta = MediaMetadata {
        width: 4000,
        height: 3000,
        capture_timestamp: 1724180200,
        camera_make: "Canon".into(),
        camera_model: "EOS R5".into(),
        lens_model: None,
        iso: Some(200),
        exposure_time: None,
        f_number: None,
        gps_latitude: Some(48.8584),  // Paris Eiffel Tower
        gps_longitude: Some(2.2945),
    };

    let media_id = engine.import_photo("vacation.jpg", "image/jpeg", b"IMAGE_DATA", meta).unwrap();

    // Owner view contains GPS
    let owner_view = engine.get_redacted_media_view(&media_id, true).unwrap();
    assert_eq!(owner_view.metadata.gps_latitude, Some(48.8584));

    // Shared public view strips GPS
    let public_view = engine.get_redacted_media_view(&media_id, false).unwrap();
    assert_eq!(public_view.metadata.gps_latitude, None);
    assert_eq!(public_view.metadata.gps_longitude, None);
    // Visual metadata remains intact
    assert_eq!(public_view.metadata.camera_make, "Canon");
    assert_eq!(public_view.metadata.iso, Some(200));
}

#[test]
fn test_r38_f_album_creation_ordering_and_deletion() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let cas = CasChunkStore::new();
    let mut engine = NexPhotosEngine::new([0xB4; 32], alice_actor, runtime, cas);

    let dummy_meta = MediaMetadata {
        width: 100, height: 100, capture_timestamp: 0,
        camera_make: "Test".into(), camera_model: "Model".into(),
        lens_model: None, iso: None, exposure_time: None, f_number: None,
        gps_latitude: None, gps_longitude: None,
    };

    let p1 = engine.import_photo("p1.jpg", "image/jpeg", b"P1", dummy_meta.clone()).unwrap();
    let p2 = engine.import_photo("p2.jpg", "image/jpeg", b"P2", dummy_meta).unwrap();

    // Create Album
    let album_id = engine.create_album("Trip", "Summer 2026", vec![p1, p2]).unwrap();
    let digest = engine.compute_album_merkle_digest(&album_id).unwrap();
    assert_ne!(digest, [0u8; 32]);

    // Delete photo p1 -> Verify removed from library and album
    engine.delete_photo(p1, None).unwrap();
    assert!(!engine.photos.contains_key(&p1));
    assert_eq!(engine.albums.get(&album_id).unwrap().media_items, vec![p2]);
}
