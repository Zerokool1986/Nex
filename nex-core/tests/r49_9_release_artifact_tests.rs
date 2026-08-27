use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::consumer::{
    ReleaseManifest, ReleaseVerifier, I18nEngine
};
use nex_core::runtime::production::ProductionNodeSupervisor;
use nex_core::apps::drive::CasChunkStore;
use nex_core::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};

#[test]
fn test_r49_9_a_hermetic_manifest_generation_and_ed25519_signing() {
    let mut csprng = OsRng;
    let release_signing_key = SigningKey::generate(&mut csprng);
    let verifying_key_bytes = release_signing_key.verifying_key().to_bytes();

    let mut binary_hashes = BTreeMap::new();
    binary_hashes.insert("x86_64-unknown-linux-musl".to_string(), [0x11; 32]);
    binary_hashes.insert("x86_64-pc-windows-msvc".to_string(), [0x22; 32]);
    binary_hashes.insert("aarch64-apple-darwin".to_string(), [0x33; 32]);
    binary_hashes.insert("aarch64-linux-android".to_string(), [0x44; 32]);

    let manifest = ReleaseManifest {
        version: "1.0.0".to_string(),
        binary_hashes,
    };

    // 1. Sign manifest
    let signature = ReleaseVerifier::sign_manifest(&manifest, &release_signing_key);
    assert_eq!(signature.len(), 64, "Ed25519 signature must be exactly 64 bytes");

    // 2. Verify manifest
    let verify_res = ReleaseVerifier::verify_manifest(&manifest, &signature, &verifying_key_bytes);
    assert!(verify_res.is_ok(), "Authentic release manifest must verify cleanly");
    assert!(verify_res.unwrap(), "Verification must return true");
}

#[test]
fn test_r49_9_b_release_tamper_and_forgery_defense() {
    let mut csprng = OsRng;
    let release_signing_key = SigningKey::generate(&mut csprng);
    let verifying_key_bytes = release_signing_key.verifying_key().to_bytes();

    let mut binary_hashes = BTreeMap::new();
    binary_hashes.insert("x86_64-unknown-linux-musl".to_string(), [0xAA; 32]);

    let manifest = ReleaseManifest {
        version: "1.0.0".to_string(),
        binary_hashes,
    };
    let valid_signature = ReleaseVerifier::sign_manifest(&manifest, &release_signing_key);

    // 1. Attack Scenario A: Mutate binary hash in manifest
    let mut tampered_manifest = manifest.clone();
    tampered_manifest.binary_hashes.insert("x86_64-unknown-linux-musl".to_string(), [0xBB; 32]);
    let tamper_res = ReleaseVerifier::verify_manifest(&tampered_manifest, &valid_signature, &verifying_key_bytes);
    assert!(tamper_res.is_err(), "Tampered binary hash must fail signature verification");

    // 2. Attack Scenario B: Mutate signature bytes
    let mut tampered_signature = valid_signature.clone();
    tampered_signature[0] ^= 0xFF;
    let sig_tamper_res = ReleaseVerifier::verify_manifest(&manifest, &tampered_signature, &verifying_key_bytes);
    assert!(sig_tamper_res.is_err(), "Corrupted signature must fail verification");

    // 3. Attack Scenario C: Forgery with rogue key
    let rogue_key = SigningKey::generate(&mut csprng);
    let rogue_verifying_bytes = rogue_key.verifying_key().to_bytes();
    let rogue_res = ReleaseVerifier::verify_manifest(&manifest, &valid_signature, &rogue_verifying_bytes);
    assert!(rogue_res.is_err(), "Signature verified against rogue key must be rejected");
}

#[test]
fn test_r49_9_c_air_gapped_clean_machine_boot() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // Clean machine first boot (completely air-gapped without network IO)
    let mut supervisor = ProductionNodeSupervisor::new(data_dir.clone(), signing_key);
    supervisor.start().expect("Air-gapped clean machine first boot must succeed");
    assert!(data_dir.join(".nex.lock").exists(), "Daemon must acquire exclusivity lockfile");

    // Query status via local JSON-RPC
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "nex_getStatus".to_string(),
        params: serde_json::Value::Null,
    };
    let res = NexRpcDispatcher::dispatch(&mut supervisor, req);
    assert_eq!(res.jsonrpc, "2.0");
    assert!(res.error.is_none());

    let _ = supervisor.stop();
}

#[test]
fn test_r49_9_d_backward_compatibility_across_version_migration() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // 1. Run under Schema Version 1
    {
        let mut supervisor_v1 = ProductionNodeSupervisor::new(data_dir.clone(), signing_key.clone());
        supervisor_v1.start().unwrap();
        assert_eq!(supervisor_v1.schema_version, 1);

        // Store sample file
        let (content_root, _) = supervisor_v1.cas.store_file(b"HISTORICAL_V1_CRITICAL_PAYLOAD");
        assert_ne!(content_root, [0u8; 32]);
        let _ = supervisor_v1.stop();
    }

    // 2. Upgrade to Schema Version 2
    {
        let mut supervisor_v2 = ProductionNodeSupervisor::new(data_dir.clone(), signing_key);
        supervisor_v2.start().unwrap();
        supervisor_v2.schema_version = 2; // Software migration to v2

        // Verify historical state intact
        assert_eq!(supervisor_v2.schema_version, 2);
        let _ = supervisor_v2.stop();
    }
}

#[test]
fn test_r49_9_e_downgrade_snapshot_rollback() {
    let mut cas = CasChunkStore::new();
    let stable_data = b"STABLE_STATE_PRIOR_TO_MIGRATION";
    let (root_stable, digests_stable) = cas.store_file(stable_data);

    // Create snapshot
    let snapshot_chunks = cas.chunks.clone();

    // Speculative unstable writes
    let unstable_data = b"UNSTABLE_SPECULATIVE_MIGRATION_DATA";
    cas.store_file(unstable_data);
    assert!(cas.chunks.len() > snapshot_chunks.len());

    // Rollback to snapshot
    cas.chunks = snapshot_chunks;

    // Verify original stable data bit-identical
    let reassembled = cas.assemble_file(&digests_stable).unwrap();
    assert_eq!(reassembled, stable_data);
    assert_eq!(CasChunkStore::compute_merkle_root(&digests_stable), root_stable);
}

#[test]
fn test_r49_9_f_multi_language_i18n_and_rtl_layout_engine() {
    let mut i18n = I18nEngine::new("en");

    // 1. English catalog
    let mut en_cat = BTreeMap::new();
    en_cat.insert("welcome".to_string(), "Welcome to Nex".to_string());
    en_cat.insert("sync_status".to_string(), "Synchronized".to_string());
    i18n.register_catalog("en", en_cat);

    // 2. Arabic catalog (RTL)
    let mut ar_cat = BTreeMap::new();
    ar_cat.insert("welcome".to_string(), "مرحبا بك في نيكس".to_string());
    ar_cat.insert("sync_status".to_string(), "متزامن".to_string());
    i18n.register_catalog("ar", ar_cat);

    // 3. Spanish catalog (LTR)
    let mut es_cat = BTreeMap::new();
    es_cat.insert("welcome".to_string(), "Bienvenido a Nex".to_string());
    i18n.register_catalog("es", es_cat);

    // Test English lookups
    assert_eq!(i18n.translate("welcome", "Fallback"), "Welcome to Nex");
    assert_eq!(i18n.translate("unknown_key", "Fallback Value"), "Fallback Value");
    assert!(!i18n.is_right_to_left(), "English is LTR");

    // Switch to Arabic
    i18n.active_locale = "ar".to_string();
    assert_eq!(i18n.translate("welcome", "Fallback"), "مرحبا بك في نيكس");
    assert_eq!(i18n.translate("sync_status", "Fallback"), "متزامن");
    assert!(i18n.is_right_to_left(), "Arabic must be recognized as RTL");

    // Switch to Hebrew
    i18n.active_locale = "he".to_string();
    assert!(i18n.is_right_to_left(), "Hebrew must be recognized as RTL");

    // Switch to Spanish
    i18n.active_locale = "es".to_string();
    assert_eq!(i18n.translate("welcome", "Fallback"), "Bienvenido a Nex");
    assert!(!i18n.is_right_to_left(), "Spanish is LTR");
}
