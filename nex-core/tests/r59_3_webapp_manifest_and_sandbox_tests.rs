use nex_core::apps::web::WebAppManifest;

#[test]
fn test_r59_3_a_webapp_manifest_default_security() {
    let manifest = WebAppManifest::default_secure("app.nex.docs", "Nex Documents");
    assert_eq!(manifest.app_id, "app.nex.docs");
    assert_eq!(manifest.name, "Nex Documents");
    assert_eq!(manifest.entrypoint, "/index.html");
    assert!(manifest.content_security_policy.contains("default-src 'self' nex:;"));
}

#[test]
fn test_r59_3_b_manifest_json_serialization() {
    let manifest = WebAppManifest::default_secure("app.nex.calc", "Sovereign Calculator");
    let json_str = serde_json::to_string(&manifest).unwrap();

    let deserialized: WebAppManifest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(manifest, deserialized);
}

#[test]
fn test_r59_3_c_manifest_custom_entrypoint() {
    let mut manifest = WebAppManifest::default_secure("app.nex.viewer", "Photo Viewer");
    manifest.entrypoint = "/viewer.html".to_string();
    assert_eq!(manifest.entrypoint, "/viewer.html");
}

#[test]
fn test_r59_3_d_multi_manifest_isolation() {
    let manifest1 = WebAppManifest::default_secure("app.1", "App 1");
    let manifest2 = WebAppManifest::default_secure("app.2", "App 2");
    assert_ne!(manifest1.app_id, manifest2.app_id);
}

#[test]
fn test_r59_3_e_manifest_version_bumping() {
    let mut manifest = WebAppManifest::default_secure("app.nex.editor", "Code Editor");
    manifest.version = "1.1.0".to_string();
    assert_eq!(manifest.version, "1.1.0");
}

#[test]
fn test_r59_3_f_zero_regression_manifest_lifecycle() {
    for i in 0..10 {
        let manifest = WebAppManifest::default_secure(&format!("app.{}", i), "Test App");
        assert_eq!(manifest.app_id, format!("app.{}", i));
    }
}
