use std::collections::BTreeMap;
use std::fs;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use nex_core::api::{NexAppApi, NexCoreRuntime, CoreRuntimeError};
use nex_core::object::types::{ObjectType, ObjectID, NamespaceID};
use nex_core::identity::types::{OP_REGISTER_LWW, OP_OBJECT_TOMBSTONE, OP_ALL, KeyType};
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r34_a_universal_object_model_across_types() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let mut runtime = NexCoreRuntime::new(alice_key, None);
    let ns = [0x11; 32];

    // 1. Create Drive Inode Object
    let mut drive_meta = BTreeMap::new();
    drive_meta.insert("path".into(), "/docs/whitepaper.pdf".into());
    drive_meta.insert("mime".into(), "application/pdf".into());
    let doc_id = runtime.create_object(ns, ObjectType::DriveInode, drive_meta, b"PDF_CAS_DATA".to_vec()).unwrap();

    // 2. Create Photo Media Object
    let mut photo_meta = BTreeMap::new();
    photo_meta.insert("camera".into(), "Sony A7IV".into());
    let photo_id = runtime.create_object(ns, ObjectType::PhotoMedia, photo_meta, b"RAW_IMAGE_BYTES".to_vec()).unwrap();

    // 3. Create Vault Secret Object
    let mut vault_meta = BTreeMap::new();
    vault_meta.insert("service".into(), "github.com".into());
    let vault_id = runtime.create_object(ns, ObjectType::VaultItem, vault_meta, b"ENCRYPTED_SECRET".to_vec()).unwrap();

    // Verify all objects stored and retrieved correctly
    let doc_obj = runtime.read_object(&doc_id).unwrap();
    assert_eq!(doc_obj.object_type, ObjectType::DriveInode);
    assert_eq!(doc_obj.payload_bytes, b"PDF_CAS_DATA");

    let photo_obj = runtime.read_object(&photo_id).unwrap();
    assert_eq!(photo_obj.object_type, ObjectType::PhotoMedia);

    let vault_obj = runtime.read_object(&vault_id).unwrap();
    assert_eq!(vault_obj.object_type, ObjectType::VaultItem);
}

#[test]
fn test_r34_b_capability_authorization_chokepoint() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let bob_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_key.verifying_key().to_bytes();
    let bob_actor = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    let mut runtime = NexCoreRuntime::new(alice_key, None);
    let ns = [0x22; 32];

    // Alice creates object
    let obj_id = runtime.create_object(ns, ObjectType::DriveInode, BTreeMap::new(), b"V1".to_vec()).unwrap();

    // Alice delegates write access to Bob for this object
    let proof_for_bob = runtime.delegate_capability(
        bob_actor,
        ns,
        Some(obj_id),
        OP_REGISTER_LWW,
        (0, 100)
    ).unwrap();

    // Bob mutates object with valid capability proof
    let mut bob_meta = BTreeMap::new();
    bob_meta.insert("modified_by".into(), "bob".into());
    let m_id = runtime.mutate_object(obj_id, Some(bob_meta), Some(b"V2_BOB".to_vec()), Some(proof_for_bob.clone())).unwrap();
    assert_ne!(m_id, [0u8; 32]);

    let updated = runtime.read_object(&obj_id).unwrap();
    assert_eq!(updated.payload_bytes, b"V2_BOB");

    // Eve (unauthorized) attempts to mutate object without valid proof -> Fails
    let mut forged_proof = proof_for_bob.clone();
    forged_proof.signature[0] ^= 0xFF; // Corrupt signature
    let res_forged = runtime.mutate_object(obj_id, None, Some(b"V3_FORGED".to_vec()), Some(forged_proof));
    assert!(matches!(res_forged, Err(CoreRuntimeError::Unauthorized(_))), "Forged proof must be rejected");

    // Eve attempts to delete object with only WRITE capability -> Fails
    let res_del = runtime.delete_object(obj_id, Some(proof_for_bob));
    assert!(matches!(res_del, Err(CoreRuntimeError::Unauthorized(_))), "Write-only token cannot perform delete");
}

#[test]
fn test_r34_c_state_persistence_and_wal_recovery() {
    let temp_dir = std::env::temp_dir().join(format!("nex_r34_wal_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&temp_dir).unwrap();
    let wal_path = temp_dir.join("runtime.wal");

    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let ns = [0x33; 32];

    let obj_id;
    {
        let mut runtime = NexCoreRuntime::new(alice_key.clone(), Some(wal_path.clone()));
        obj_id = runtime.create_object(ns, ObjectType::DriveInode, BTreeMap::new(), b"DURABLE_DATA_1".to_vec()).unwrap();
        runtime.mutate_object(obj_id, None, Some(b"DURABLE_DATA_2".to_vec()), None).unwrap();
        // Drop runtime to simulate process crash
    }

    // Recover from WAL
    let recovered_mutations = nex_core::storage::wal::WriteAheadLog::recover(&wal_path).unwrap();
    assert_eq!(recovered_mutations.len(), 2, "Both committed mutations must be recovered from WAL");

    let mut fresh_runtime = NexCoreRuntime::new(alice_key, None);
    for m in recovered_mutations {
        fresh_runtime.state_node.ingest_mutation(m);
    }
    assert_eq!(fresh_runtime.state_node.dag.len(), 2);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_r34_e_tombstone_lifecycle() {
    let mut csprng = OsRng;
    let alice_key = SigningKey::generate(&mut csprng);
    let mut runtime = NexCoreRuntime::new(alice_key, None);
    let ns = [0x44; 32];

    let obj_id = runtime.create_object(ns, ObjectType::ChatMessage, BTreeMap::new(), b"HELLO".to_vec()).unwrap();
    assert!(runtime.read_object(&obj_id).is_ok());

    // Delete object
    runtime.delete_object(obj_id, None).unwrap();

    // Read fails with ObjectTombstoned
    assert_eq!(runtime.read_object(&obj_id), Err(CoreRuntimeError::ObjectTombstoned));

    // Subsequent mutate fails with ObjectTombstoned
    let res = runtime.mutate_object(obj_id, None, Some(b"REVIVE".to_vec()), None);
    assert_eq!(res, Err(CoreRuntimeError::ObjectTombstoned));
}
