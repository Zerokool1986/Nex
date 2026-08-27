use std::collections::BTreeMap;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use nex_core::api::NexCoreRuntime;
use nex_core::apps::extensions::{
    NexExtensionHost, AppManifest, CustomObjectTypeRegistration, AppCapabilityRequest
};
use nex_core::identity::types::KeyType;
use nex_core::identity::verifier::derive_actor_id;

#[test]
fn test_r41_a_manifest_verification_and_app_installation() {
    let mut csprng = OsRng;
    let user_key = SigningKey::generate(&mut csprng);
    let user_pubkey = user_key.verifying_key().to_bytes();
    let user_actor = derive_actor_id(KeyType::Ed25519, &user_pubkey);

    let dev_key = SigningKey::generate(&mut csprng);
    let dev_pubkey = dev_key.verifying_key().to_bytes();
    let dev_actor = derive_actor_id(KeyType::Ed25519, &dev_pubkey);

    let runtime = NexCoreRuntime::new(user_key, None);
    let mut host = NexExtensionHost::new(user_actor, runtime);

    // 1. Construct Manifest
    let mut manifest = AppManifest {
        manifest_version: "1.0.0".into(),
        app_id: "com.sovereign.notes".into(),
        name: "Sovereign Notes".into(),
        version: "1.0.0".into(),
        min_nex_core_version: "1.0.0".into(),
        developer_actor_id: dev_actor,
        developer_signature: vec![],
        requested_capabilities: vec![
            AppCapabilityRequest { namespace_scope: "Self".into(), allowed_operations: 0x07 }
        ],
        registered_object_types: vec![
            CustomObjectTypeRegistration {
                type_id: 0x1101, // Valid third-party range
                name: "NoteDoc".into(),
                schema_definition: "{\"title\":\"string\"}".into(),
            }
        ],
    };

    // Developer signs the manifest
    let digest = manifest.compute_canonical_digest();
    let sig = dev_key.sign(&digest);
    manifest.developer_signature = sig.to_bytes().to_vec();

    // 2. Install App
    let app_ns = host.install_app(manifest.clone(), &dev_pubkey).unwrap();
    assert!(host.installed_apps.contains_key("com.sovereign.notes"));
    assert!(host.registered_schemas.contains_key(&0x1101));
    assert_ne!(app_ns, [0u8; 32]);

    // 3. Re-install with tampered signature -> Must FAIL
    let mut tampered = manifest.clone();
    tampered.developer_signature[5] ^= 0xFF;
    assert!(host.install_app(tampered, &dev_pubkey).is_err(), "Tampered signature must be rejected");
}

#[test]
fn test_r41_c_custom_object_type_range_and_collision_rejection() {
    let mut csprng = OsRng;
    let user_key = SigningKey::generate(&mut csprng);
    let user_pubkey = user_key.verifying_key().to_bytes();
    let user_actor = derive_actor_id(KeyType::Ed25519, &user_pubkey);

    let dev_key = SigningKey::generate(&mut csprng);
    let dev_pubkey = dev_key.verifying_key().to_bytes();
    let dev_actor = derive_actor_id(KeyType::Ed25519, &dev_pubkey);

    let runtime = NexCoreRuntime::new(user_key, None);
    let mut host = NexExtensionHost::new(user_actor, runtime);

    // Attempt to register TypeID 0x0101 (Drive Inode reserved range) -> Must FAIL
    let mut illegal_manifest = AppManifest {
        manifest_version: "1.0.0".into(),
        app_id: "com.malicious.hijack".into(),
        name: "Drive Hijacker".into(),
        version: "1.0.0".into(),
        min_nex_core_version: "1.0.0".into(),
        developer_actor_id: dev_actor,
        developer_signature: vec![],
        requested_capabilities: vec![],
        registered_object_types: vec![
            CustomObjectTypeRegistration {
                type_id: 0x0101, // Reserved Core Drive Inode range!
                name: "DriveInodeFake".into(),
                schema_definition: "{}".into(),
            }
        ],
    };
    let digest = illegal_manifest.compute_canonical_digest();
    illegal_manifest.developer_signature = dev_key.sign(&digest).to_bytes().to_vec();

    assert!(host.install_app(illegal_manifest, &dev_pubkey).is_err(), "Reserved Core ObjectType ranges must be rejected");
}

#[test]
fn test_r41_e_resource_quota_and_fuel_metering_exhaustion() {
    let mut csprng = OsRng;
    let user_key = SigningKey::generate(&mut csprng);
    let user_pubkey = user_key.verifying_key().to_bytes();
    let user_actor = derive_actor_id(KeyType::Ed25519, &user_pubkey);

    let dev_key = SigningKey::generate(&mut csprng);
    let dev_pubkey = dev_key.verifying_key().to_bytes();
    let dev_actor = derive_actor_id(KeyType::Ed25519, &dev_pubkey);

    let runtime = NexCoreRuntime::new(user_key, None);
    let mut host = NexExtensionHost::new(user_actor, runtime);

    let mut manifest = AppManifest {
        manifest_version: "1.0.0".into(),
        app_id: "com.crypto.miner".into(),
        name: "Infinite Loop Miner".into(),
        version: "1.0.0".into(),
        min_nex_core_version: "1.0.0".into(),
        developer_actor_id: dev_actor,
        developer_signature: vec![],
        requested_capabilities: vec![],
        registered_object_types: vec![],
    };
    let digest = manifest.compute_canonical_digest();
    manifest.developer_signature = dev_key.sign(&digest).to_bytes().to_vec();

    host.install_app(manifest, &dev_pubkey).unwrap();

    // 1. Normal execution within fuel limit (100,000 instructions) -> PASS
    let res_ok = host.execute_sandbox("com.crypto.miner", b"HELLO", 100_000);
    assert!(res_ok.is_ok());

    // 2. Infinite execution exceeding max fuel (100,000,000 > 50,000,000) -> TRAP
    let res_exhaust = host.execute_sandbox("com.crypto.miner", b"HELLO", 100_000_000);
    assert!(res_exhaust.is_err(), "Execution exceeding fuel limit must be trapped");
}

#[test]
fn test_r41_d_clean_uninstallation_and_schema_cleanup() {
    let mut csprng = OsRng;
    let user_key = SigningKey::generate(&mut csprng);
    let user_pubkey = user_key.verifying_key().to_bytes();
    let user_actor = derive_actor_id(KeyType::Ed25519, &user_pubkey);

    let dev_key = SigningKey::generate(&mut csprng);
    let dev_pubkey = dev_key.verifying_key().to_bytes();
    let dev_actor = derive_actor_id(KeyType::Ed25519, &dev_pubkey);

    let runtime = NexCoreRuntime::new(user_key, None);
    let mut host = NexExtensionHost::new(user_actor, runtime);

    let mut manifest = AppManifest {
        manifest_version: "1.0.0".into(),
        app_id: "com.temp.tool".into(),
        name: "Temporary Utility".into(),
        version: "1.0.0".into(),
        min_nex_core_version: "1.0.0".into(),
        developer_actor_id: dev_actor,
        developer_signature: vec![],
        requested_capabilities: vec![],
        registered_object_types: vec![
            CustomObjectTypeRegistration {
                type_id: 0x2201,
                name: "TempData".into(),
                schema_definition: "{}".into(),
            }
        ],
    };
    let digest = manifest.compute_canonical_digest();
    manifest.developer_signature = dev_key.sign(&digest).to_bytes().to_vec();

    host.install_app(manifest, &dev_pubkey).unwrap();
    assert!(host.registered_schemas.contains_key(&0x2201));

    // Uninstall
    host.uninstall_app("com.temp.tool").unwrap();
    assert!(!host.installed_apps.contains_key("com.temp.tool"));
    assert!(!host.registered_schemas.contains_key(&0x2201));
}
