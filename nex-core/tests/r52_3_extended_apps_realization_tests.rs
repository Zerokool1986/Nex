use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::apps::drive::CasChunkStore;
use nex_core::apps::photos::{NexPhotosEngine, MediaMetadata};
use nex_core::apps::vault::{NexVaultEngine, VaultCategory};
use nex_core::apps::backup::NexBackupEngine;
use nex_core::object::types::ObjectType;

#[test]
fn test_r52_3_a_photos_ingestion_and_metadata_redaction() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_photos = [0xFA; 32];
    let cas = CasChunkStore::new();
    let mut engine = NexPhotosEngine::new(ns_photos, node.identity.actor_id, &mut node, cas);

    let meta = MediaMetadata {
        width: 3840,
        height: 2160,
        capture_timestamp: 1724200000,
        camera_make: "Sony".to_string(),
        camera_model: "A7IV".to_string(),
        lens_model: Some("FE 24-70mm F2.8 GM II".to_string()),
        iso: Some(100),
        exposure_time: Some("1/250".to_string()),
        f_number: Some(2.8),
        gps_latitude: Some(37.7749),
        gps_longitude: Some(-122.4194),
    };

    let raw_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
    let photo_id = engine.import_photo("golden_gate.jpg", "image/jpeg", &raw_jpeg, meta).unwrap();

    // 1. Full view with GPS
    let full_view = engine.get_redacted_media_view(&photo_id, true).unwrap();
    assert_eq!(full_view.metadata.gps_latitude, Some(37.7749));

    // 2. Redacted view without GPS
    let redacted_view = engine.get_redacted_media_view(&photo_id, false).unwrap();
    assert_eq!(redacted_view.metadata.gps_latitude, None);
    assert_eq!(redacted_view.metadata.gps_longitude, None);
    assert_eq!(redacted_view.metadata.camera_model, "A7IV");
}

#[test]
fn test_r52_3_b_photos_album_creation_and_merkle_digest() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_photos = [0xFA; 32];
    let cas = CasChunkStore::new();
    let mut engine = NexPhotosEngine::new(ns_photos, node.identity.actor_id, &mut node, cas);

    let dummy_meta = MediaMetadata {
        width: 1920,
        height: 1080,
        capture_timestamp: 1724200000,
        camera_make: "Nikon".to_string(),
        camera_model: "Z8".to_string(),
        lens_model: None,
        iso: None,
        exposure_time: None,
        f_number: None,
        gps_latitude: None,
        gps_longitude: None,
    };

    let p1 = engine.import_photo("photo1.jpg", "image/jpeg", b"RAW1", dummy_meta.clone()).unwrap();
    let p2 = engine.import_photo("photo2.jpg", "image/jpeg", b"RAW2", dummy_meta.clone()).unwrap();

    let album_id = engine.create_album("Summer Vacation", "2026 Trip", vec![p1, p2]).unwrap();
    let digest1 = engine.compute_album_merkle_digest(&album_id).unwrap();
    assert_ne!(digest1, [0u8; 32]);

    let p3 = engine.import_photo("photo3.jpg", "image/jpeg", b"RAW3", dummy_meta).unwrap();
    engine.add_photo_to_album(album_id, p3).unwrap();
    let digest2 = engine.compute_album_merkle_digest(&album_id).unwrap();

    assert_ne!(digest1, digest2, "Album Merkle digest must update upon adding new media items");
}

#[test]
fn test_r52_3_c_vault_e2ee_secret_storage_and_decryption() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_vault = [0xEE; 32];
    let master_key = [0x42; 32];
    let mut vault = NexVaultEngine::new(ns_vault, &mut node);

    let secret = b"my_super_secure_master_password_2026";
    let item_id = vault.store_item("Email Account", VaultCategory::Login, secret, &master_key, None).unwrap();

    // Read back and decrypt
    let (item, decrypted) = vault.read_item(&item_id, &master_key).unwrap();
    assert_eq!(item.title, "Email Account");
    assert_eq!(item.category, VaultCategory::Login);
    assert_eq!(decrypted, secret);
}

#[test]
fn test_r52_3_d_vault_tamper_rejection_and_tombstoning() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_vault = [0xEE; 32];
    let master_key = [0x42; 32];
    let wrong_key = [0x99; 32];
    let mut vault = NexVaultEngine::new(ns_vault, &mut node);

    let secret = b"sensitive_financial_note";
    let item_id = vault.store_item("Crypto Recovery Seed", VaultCategory::CryptoKey, secret, &master_key, None).unwrap();

    // Decryption with wrong key must fail MAC validation
    let err = vault.read_item(&item_id, &wrong_key);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("MAC validation failure"));

    // Delete item
    assert!(vault.delete_item(item_id, None).is_ok());
    assert_eq!(vault.list_items().len(), 0);
}

#[test]
fn test_r52_3_e_backup_deduplicated_snapshot_and_restore() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_backup = [0xBB; 32];
    let cas = CasChunkStore::new();
    let mut backup_engine = NexBackupEngine::new(ns_backup, &mut node, cas);

    let sample_fs_data = b"Sovereign filesystem state archive payload representing 100 files.";
    let snapshot_id = backup_engine.create_backup("Daily Backup 2026-08-21", sample_fs_data, 1, None).unwrap();

    // Restore
    let restored = backup_engine.restore_backup(&snapshot_id).unwrap();
    assert_eq!(restored, sample_fs_data);

    // Deduplicated identical second backup
    let snapshot_id2 = backup_engine.create_backup("Duplicate Backup", sample_fs_data, 2, None).unwrap();
    assert_ne!(snapshot_id, snapshot_id2);

    let snap1 = backup_engine.snapshots.get(&snapshot_id).unwrap();
    let snap2 = backup_engine.snapshots.get(&snapshot_id2).unwrap();
    assert_eq!(snap1.content_root, snap2.content_root, "Identical content must produce identical CAS roots");
}

#[test]
fn test_r52_3_f_full_ecosystem_multi_app_merkle_seal() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let mut node = NexNode::new(tmp.path(), SigningKey::generate(&mut csprng));
    node.start().unwrap();

    let ns_drive = [0xD1; 32];
    let ns_chat = [0xC1; 32];
    let ns_comm = [0xB1; 32];
    let ns_photos = [0xA1; 32];
    let ns_vault = [0xEE; 32];
    let ns_backup = [0xBB; 32];

    // Author across all 6 applications
    node.create_object(ns_drive, ObjectType::DriveInode, BTreeMap::new(), b"Drive Data".to_vec()).unwrap();
    node.create_object(ns_chat, ObjectType::ChatMessage, BTreeMap::new(), b"Chat Data".to_vec()).unwrap();
    node.create_object(ns_comm, ObjectType::Community, BTreeMap::new(), b"Community Data".to_vec()).unwrap();
    node.create_object(ns_photos, ObjectType::PhotoMedia, BTreeMap::new(), b"Photos Data".to_vec()).unwrap();
    node.create_object(ns_vault, ObjectType::VaultItem, BTreeMap::new(), b"Vault Data".to_vec()).unwrap();
    node.create_object(ns_backup, ObjectType::BackupIndex, BTreeMap::new(), b"Backup Data".to_vec()).unwrap();

    assert_eq!(node.state.object_store.len(), 6);

    let cp = node.sync_now().unwrap();
    assert_ne!(cp.body.state_root, [0u8; 32]);

    node.stop().unwrap();
}
