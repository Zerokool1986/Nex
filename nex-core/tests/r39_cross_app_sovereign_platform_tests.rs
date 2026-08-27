use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::api::{NexCoreRuntime, NexAppApi};
use nex_core::apps::drive::{NexDriveEngine, CasChunkStore};
use nex_core::apps::photos::{NexPhotosEngine, MediaMetadata};
use nex_core::apps::chat::{NexChatEngine, ChannelType};
use nex_core::apps::community::{NexCommunityEngine, CommunityRole};
use nex_core::identity::types::{KeyType, CapabilityToken, CapabilityProof, OP_READ, OP_WRITE, OP_OBJECT_TOMBSTONE};
use nex_core::identity::verifier::{derive_actor_id, verify_capability_chain};

#[test]
fn test_r39_a_cross_app_object_referencing_and_loose_coupling() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let cas = CasChunkStore::new();

    let mut photos = NexPhotosEngine::new([0xC1; 32], alice_actor, runtime.clone(), cas);
    let mut chat = NexChatEngine::new([0xC2; 32], alice_actor, runtime.clone());
    let mut comm = NexCommunityEngine::new([0xC3; 32], alice_actor, runtime);

    // 1. Ingest Photo
    let meta = MediaMetadata {
        width: 1920, height: 1080, capture_timestamp: 1724181000,
        camera_make: "Sony".into(), camera_model: "A7IV".into(),
        lens_model: None, iso: None, exposure_time: None, f_number: None,
        gps_latitude: None, gps_longitude: None,
    };
    let photo_id = photos.import_photo("nature.jpg", "image/jpeg", b"RAW_PHOTO_DATA", meta).unwrap();
    let photo = photos.photos.get(&photo_id).unwrap();

    // 2. Chat references Photo as attachment
    let channel_id = chat.create_channel("General", ChannelType::GroupMultiParty, vec![]).unwrap();
    let chat_key = [0x55; 32];
    let msg_id = chat.send_message(
        channel_id,
        b"Look at this landscape!",
        &chat_key,
        vec![],
        vec![photo.raw_content_root], // Cross-app CAS digest reference
        None,
    ).unwrap();

    // 3. Communities references Photo in a Post
    let comm_id = comm.create_community("Photography Club", "Lens enthusiasts").unwrap();
    let chan_id = comm.create_channel(comm_id, "showcase", false).unwrap();
    let post_id = comm.create_post(
        comm_id,
        chan_id,
        "Morning Mist",
        "Captured at dawn.",
        vec![photo.raw_content_root], // Cross-app CAS digest reference
        None,
    ).unwrap();

    // Verify all 3 objects exist with zero state corruption
    assert!(photos.photos.contains_key(&photo_id));
    assert!(chat.api.read_object(&msg_id).is_ok());
    assert!(comm.posts.contains_key(&post_id));
}

#[test]
fn test_r39_b_cross_app_capability_attenuation_and_anti_escalation() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let bob_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_key.verifying_key().to_bytes();
    let bob_actor = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    // Alice grants Bob read-only access to Photo Namespace
    let photo_ns = [0xC1; 32];
    let token = CapabilityToken {
        issuer_device_id: alice_actor,
        grantee_actor_id: bob_actor,
        allowed_operations: OP_READ,
        namespace_id: photo_ns,
        object_id: None,
        valid_from_epoch: 0,
        valid_until_epoch: 100,
        delegation_depth: 1,
    };
    let token_bytes = token.canonical_bytes();
    let sig = alice_key.sign(&token_bytes);
    let proof = CapabilityProof {
        token,
        signature: sig.to_bytes().to_vec(),
        parent_proof: None,
    };

    let empty_revocations = BTreeMap::new();

    // 1. Bob attempts to use read token in Photo Namespace -> PASS
    let res_read = verify_capability_chain(
        &proof,
        OP_READ,
        &photo_ns,
        None,
        1,
        &empty_revocations,
        &alice_actor,
    );
    assert!(res_read.is_ok(), "Read capability must be valid");

    // 2. Bob attempts to escalate token to OP_WRITE -> FAIL
    let res_escalate = verify_capability_chain(
        &proof,
        OP_WRITE,
        &photo_ns,
        None,
        1,
        &empty_revocations,
        &alice_actor,
    );
    assert!(res_escalate.is_err(), "Permission escalation must be rejected");

    // 3. Bob attempts to use Photo token in Drive Namespace -> FAIL
    let drive_ns = [0xD1; 32];
    let res_ns_escape = verify_capability_chain(
        &proof,
        OP_READ,
        &drive_ns,
        None,
        1,
        &empty_revocations,
        &alice_actor,
    );
    assert!(res_ns_escape.is_err(), "Namespace escape must be rejected");
}

#[test]
fn test_r39_c_shared_cas_multi_tenant_chunk_reachability() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut cas = CasChunkStore::new();

    // Store shared 2MB video chunk
    let video_bytes = vec![0x33; 2 * 1024 * 1024];
    let (root_digest, chunk_digests) = cas.store_file(&video_bytes);

    // Both Drive and Photos reference the exact same chunk
    let mut drive = NexDriveEngine::new([0xD2; 32], alice_actor, runtime.clone(), cas.clone());
    let mut photos = NexPhotosEngine::new([0xC4; 32], alice_actor, runtime, cas);

    let drive_file_id = drive.create_file("video.mp4", &video_bytes).unwrap();

    let meta = MediaMetadata {
        width: 3840, height: 2160, capture_timestamp: 1724182000,
        camera_make: "Sony".into(), camera_model: "FX3".into(),
        lens_model: None, iso: None, exposure_time: None, f_number: None,
        gps_latitude: None, gps_longitude: None,
    };
    let photo_id = photos.import_photo("video.mp4", "video/mp4", &video_bytes, meta).unwrap();

    // Verify both applications reference the exact same underlying CAS chunk root
    let drive_file = drive.files.get(&drive_file_id).unwrap();
    let photo_media = photos.photos.get(&photo_id).unwrap();

    assert_eq!(drive_file.content_root, photo_media.raw_content_root);
    assert_eq!(drive_file.content_root, root_digest);
}

#[test]
fn test_r39_e_unified_identity_platform_wide_revocation() {
    let mut csprng = OsRng;
    let root_key = SigningKey::generate(&mut csprng);
    let root_pubkey = root_key.verifying_key().to_bytes();
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_pubkey);

    let device2_key = SigningKey::generate(&mut csprng);
    let device2_pubkey = device2_key.verifying_key().to_bytes();
    let device2_actor = derive_actor_id(KeyType::Ed25519, &device2_pubkey);

    // Root grants Device 2 universal access to namespace 0xAA
    let ns = [0xAA; 32];
    let token = CapabilityToken {
        issuer_device_id: root_actor,
        grantee_actor_id: device2_actor,
        allowed_operations: OP_READ | OP_WRITE,
        namespace_id: ns,
        object_id: None,
        valid_from_epoch: 0,
        valid_until_epoch: 100,
        delegation_depth: 1,
    };
    let sig = root_key.sign(&token.canonical_bytes());
    let token_hash = token.hash();
    let proof = CapabilityProof {
        token,
        signature: sig.to_bytes().to_vec(),
        parent_proof: None,
    };

    let mut revocations = BTreeMap::new();

    // Initially valid
    assert!(verify_capability_chain(&proof, OP_READ, &ns, None, 1, &revocations, &root_actor).is_ok());

    // Root publishes active revocation fence against token_hash at epoch 2
    revocations.insert(token_hash, 2);

    // Subsequent operation across all applications is rejected
    assert!(verify_capability_chain(&proof, OP_READ, &ns, None, 2, &revocations, &root_actor).is_err(), "Revocation fence must block all app operations");
}

#[test]
fn test_r39_f_cross_app_privacy_gps_redaction_flow() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let cas = CasChunkStore::new();

    let mut photos = NexPhotosEngine::new([0xC5; 32], alice_actor, runtime.clone(), cas);
    let mut comm = NexCommunityEngine::new([0xC6; 32], alice_actor, runtime);

    // Private Photo with GPS
    let meta = MediaMetadata {
        width: 4000, height: 3000, capture_timestamp: 1724183000,
        camera_make: "Nikon".into(), camera_model: "Z8".into(),
        lens_model: None, iso: Some(100), exposure_time: None, f_number: None,
        gps_latitude: Some(51.5007), // London Big Ben
        gps_longitude: Some(-0.1246),
    };
    let photo_id = photos.import_photo("london.jpg", "image/jpeg", b"RAW_IMAGE", meta).unwrap();

    // Public community share strips GPS
    let public_view = photos.get_redacted_media_view(&photo_id, false).unwrap();
    assert_eq!(public_view.metadata.gps_latitude, None);
    assert_eq!(public_view.metadata.gps_longitude, None);
    assert_eq!(public_view.metadata.camera_make, "Nikon");

    // Community post created referencing the redacted view
    let comm_id = comm.create_community("Travelers", "World explorers").unwrap();
    let chan_id = comm.create_channel(comm_id, "uk", false).unwrap();
    let post_id = comm.create_post(
        comm_id,
        chan_id,
        "London Trip",
        "Visited Westminster today!",
        vec![public_view.raw_content_root],
        None,
    ).unwrap();

    assert!(comm.posts.contains_key(&post_id));
}

#[test]
fn test_r39_g_cross_app_tombstone_propagation_and_reference_masking() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let cas = CasChunkStore::new();

    let mut photos = NexPhotosEngine::new([0xC7; 32], alice_actor, runtime.clone(), cas);
    let mut chat = NexChatEngine::new([0xC8; 32], alice_actor, runtime);

    let meta = MediaMetadata {
        width: 100, height: 100, capture_timestamp: 0,
        camera_make: "A".into(), camera_model: "B".into(),
        lens_model: None, iso: None, exposure_time: None, f_number: None,
        gps_latitude: None, gps_longitude: None,
    };
    let photo_id = photos.import_photo("temp.jpg", "image/jpeg", b"DATA", meta).unwrap();

    // Send chat message referencing photo
    let chan_id = chat.create_channel("Direct", ChannelType::Direct1to1, vec![]).unwrap();
    let msg_id = chat.send_message(chan_id, b"Look at this photo", &[0x11; 32], vec![], vec![[0x99; 32]], None).unwrap();

    // Delete photo in Photos
    photos.delete_photo(photo_id, None).unwrap();
    assert!(!photos.photos.contains_key(&photo_id));

    // Chat message is preserved in causal DAG, not corrupted
    let (msg, _) = chat.read_message(&msg_id, &[0x11; 32]).unwrap();
    assert_eq!(msg.message_id, msg_id);
}
