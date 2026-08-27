use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::api::{NexCoreRuntime, NexAppApi};
use nex_core::apps::drive::{NexDriveEngine, CasChunkStore, CHUNK_SIZE_2MB};
use nex_core::identity::types::{OP_REGISTER_LWW, OP_OBJECT_TOMBSTONE, KeyType};
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r35_a_drive_object_semantics_and_metadata() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let runtime = NexCoreRuntime::new(alice_key, None);
    let ns = [0x55; 32];

    let mut drive = NexDriveEngine::new(ns, runtime);
    let file_content = b"HELLO_NEX_SOVEREIGN_DRIVE_V1";
    let obj_id = drive.upload_file("/docs/manifesto.txt", "text/plain", file_content, None).unwrap();

    let downloaded = drive.download_file(&obj_id).unwrap();
    let obj = drive.api.read_object(&obj_id).unwrap();
    let inode: nex_core::apps::drive::DriveInode = serde_json::from_slice(&obj.payload_bytes).unwrap();
    assert_eq!(inode.name, "manifesto.txt");
    assert_eq!(inode.size_bytes, file_content.len() as u64);
    assert_eq!(downloaded, file_content);
}

#[test]
fn test_r35_b_directory_merkle_tree_determinism() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let runtime_a = NexCoreRuntime::new(alice_key.clone(), None);
    let runtime_b = NexCoreRuntime::new(alice_key, None);
    let ns = [0x66; 32];

    let mut drive_a = NexDriveEngine::new(ns, runtime_a);
    let mut drive_b = NexDriveEngine::new(ns, runtime_b);

    // Upload same files in different insertion orders
    let f1 = b"CONTENT_A";
    let f2 = b"CONTENT_B";

    drive_a.upload_file("/file1.txt", "text/plain", f1, None).unwrap();
    drive_a.upload_file("/file2.txt", "text/plain", f2, None).unwrap();

    drive_b.upload_file("/file2.txt", "text/plain", f2, None).unwrap();
    drive_b.upload_file("/file1.txt", "text/plain", f1, None).unwrap();

    let digest_a = drive_a.compute_directory_merkle_digest("/");
    let digest_b = drive_b.compute_directory_merkle_digest("/");

    assert_eq!(digest_a, digest_b, "Directory Merkle digests must be deterministic regardless of insertion order");
}

#[test]
fn test_r35_c_cas_blob_store_multi_chunk_and_deduplication() {
    let mut cas = CasChunkStore::new();

    // Create 5MB payload (3 chunks: 2MB + 2MB + 1MB)
    let large_file = vec![0x42; 5 * 1024 * 1024];
    let (root_1, digests_1) = cas.store_file(&large_file);
    assert_eq!(digests_1.len(), 3);

    // Reassemble and verify
    let reassembled = cas.assemble_file(&digests_1).unwrap();
    assert_eq!(reassembled.len(), 5 * 1024 * 1024);
    assert_eq!(reassembled, large_file);

    // Store duplicate file -> Verifies deduplication (no extra chunks created)
    let initial_chunk_count = cas.chunks.len();
    let (root_2, digests_2) = cas.store_file(&large_file);
    assert_eq!(root_1, root_2);
    assert_eq!(digests_1, digests_2);
    assert_eq!(cas.chunks.len(), initial_chunk_count, "Identical content must be 100% deduplicated");
}

#[test]
fn test_r35_d_path_traversal_sanitization() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let runtime = NexCoreRuntime::new(alice_key, None);
    let mut drive = NexDriveEngine::new([0x77; 32], runtime);

    // Reject path traversals
    assert!(drive.upload_file("/../etc/passwd", "text/plain", b"MALICIOUS", None).is_err());
    assert!(drive.upload_file("/foo/bar/../../secret", "text/plain", b"MALICIOUS", None).is_err());
    assert!(drive.upload_file("/foo\0bar", "text/plain", b"MALICIOUS", None).is_err());
}

#[test]
fn test_r35_e_capability_enforcement_and_deletion() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let bob_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_key.verifying_key().to_bytes();
    let bob_actor = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    let runtime = NexCoreRuntime::new(alice_key, None);
    let ns = [0x88; 32];
    let mut drive = NexDriveEngine::new(ns, runtime);

    let obj_id = drive.upload_file("/secret.txt", "text/plain", b"TOP_SECRET", None).unwrap();

    // Alice delegates DELETE capability for this file to Bob
    let del_proof = drive.api.delegate_capability(
        bob_actor,
        ns,
        Some(obj_id),
        OP_OBJECT_TOMBSTONE,
        (0, 100),
    ).unwrap();

    // Delete file using valid capability
    drive.delete_file("/secret.txt", obj_id, Some(del_proof)).unwrap();

    // Download should fail because object is tombstoned
    assert!(drive.download_file(&obj_id).is_err());
}
