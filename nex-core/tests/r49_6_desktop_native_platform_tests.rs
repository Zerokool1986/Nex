use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use nex_core::runtime::production::ProductionNodeSupervisor;
use nex_core::runtime::consumer::{
    DesktopPlatformManager, TrayState, TrayAction
};
use nex_core::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};
use nex_core::apps::drive::CasChunkStore;

#[test]
fn test_r49_6_a_clean_os_first_boot_and_keyring_setup() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let mut supervisor = ProductionNodeSupervisor::new(data_dir.clone(), signing_key);
    supervisor.start().expect("Clean OS first boot must succeed");
    assert!(data_dir.join(".nex.lock").exists(), "Daemon must acquire exclusivity lockfile");

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "nex_getStatus".to_string(),
        params: serde_json::Value::Null,
    };
    let res = NexRpcDispatcher::dispatch(&mut supervisor, req);
    assert_eq!(res.jsonrpc, "2.0");
    assert!(res.error.is_none());
    let result_obj = res.result.unwrap();
    assert_eq!(result_obj["schema_version"], 1);

    let _ = supervisor.stop();
}

#[test]
fn test_r49_6_b_native_file_chooser_50mb_cas_import() {
    let mut desktop_mgr = DesktopPlatformManager::new();

    // Create 50MB test payload (25 chunks of 2MB)
    let payload = vec![0xEEu8; 50 * 1024 * 1024];

    let start = std::time::Instant::now();
    let (content_root, chunk_digests) = desktop_mgr.import_native_file(&payload);
    let elapsed = start.elapsed();

    assert_eq!(chunk_digests.len(), 25, "50MB file must produce exactly 25 chunks of 2MB");
    assert_eq!(CasChunkStore::compute_merkle_root(&chunk_digests), content_root);
    assert!(elapsed.as_millis() < 2000, "50MB native file CAS import must complete rapidly (took {}ms)", elapsed.as_millis());

    // Verify 50MB file reassembly
    let reassembled = desktop_mgr.cas.assemble_file(&chunk_digests).expect("50MB assembly must succeed");
    assert_eq!(reassembled.len(), payload.len());
    assert_eq!(reassembled, payload, "Reassembled 50MB payload must match bit-for-bit");
}

#[test]
fn test_r49_6_c_desktop_local_ipc_handshake() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let mut supervisor = ProductionNodeSupervisor::new(tmp.path().to_path_buf(), signing_key);
    supervisor.start().unwrap();

    // 1. Valid JSON-RPC status request
    let req1 = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 101,
        method: "nex_getStatus".to_string(),
        params: serde_json::Value::Null,
    };
    let res1 = NexRpcDispatcher::dispatch(&mut supervisor, req1);
    assert_eq!(res1.id, 101);
    assert!(res1.result.is_some());

    // 2. Unknown method request -> MethodNotFound error code (-32601)
    let req2 = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 102,
        method: "nex_unknownSubsystem".to_string(),
        params: serde_json::Value::Null,
    };
    let res2 = NexRpcDispatcher::dispatch(&mut supervisor, req2);
    assert_eq!(res2.id, 102);
    assert_eq!(res2.error.unwrap().code, -32601);

    let _ = supervisor.stop();
}

#[test]
fn test_r49_6_d_system_tray_state_synchronization() {
    let mut desktop_mgr = DesktopPlatformManager::new();
    assert_eq!(desktop_mgr.tray_state, TrayState::Running);
    assert!(!desktop_mgr.is_gui_open);
    assert!(!desktop_mgr.is_sync_paused);

    // 1. Tray action: Open GUI
    let msg1 = desktop_mgr.handle_tray_action(TrayAction::OpenGui);
    assert_eq!(msg1, Some("GUI_WINDOW_OPENED".to_string()));
    assert!(desktop_mgr.is_gui_open);

    // 2. Tray action: Pause Sync
    let msg2 = desktop_mgr.handle_tray_action(TrayAction::PauseSync);
    assert_eq!(msg2, Some("SYNC_PAUSED".to_string()));
    assert!(desktop_mgr.is_sync_paused);
    assert_eq!(desktop_mgr.tray_state, TrayState::Paused);

    // 3. Tray action: Resume Sync
    let msg3 = desktop_mgr.handle_tray_action(TrayAction::ResumeSync);
    assert_eq!(msg3, Some("SYNC_RESUMED".to_string()));
    assert!(!desktop_mgr.is_sync_paused);
    assert_eq!(desktop_mgr.tray_state, TrayState::Running);

    // 4. Tray action: Exit
    let msg4 = desktop_mgr.handle_tray_action(TrayAction::Exit);
    assert_eq!(msg4, Some("DAEMON_EXIT_REQUESTED".to_string()));
}

#[test]
fn test_r49_6_e_graceful_os_shutdown_and_sub_500ms_wal_flush() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let mut supervisor = ProductionNodeSupervisor::new(data_dir.clone(), signing_key);
    supervisor.start().unwrap();

    // Trigger graceful shutdown
    let flush_time_ms = DesktopPlatformManager::handle_graceful_shutdown(&mut supervisor)
        .expect("Graceful shutdown must complete within 500ms SLA");

    assert!(flush_time_ms < 500, "Shutdown and WAL flush must complete in <500ms (took {}ms)", flush_time_ms);
    assert!(!data_dir.join(".nex.lock").exists(), "Exclusivity lockfile must be cleanly unlinked upon shutdown");
}

#[test]
fn test_r49_6_f_multi_app_desktop_shell_sandboxing() {
    // 1. Authorized app calls
    let res_drive = DesktopPlatformManager::dispatch_webview_rpc(
        "drive",
        "drive_listDirectory",
        serde_json::json!({ "path": "/" }),
    );
    assert!(res_drive.is_ok());

    let res_chat = DesktopPlatformManager::dispatch_webview_rpc(
        "chat",
        "chat_postMessage",
        serde_json::json!({ "channel": "general" }),
    );
    assert!(res_chat.is_ok());

    // 2. Cross-app namespace violation: Drive invoking chat method
    let res_viol1 = DesktopPlatformManager::dispatch_webview_rpc(
        "drive",
        "chat_postMessage",
        serde_json::json!({ "channel": "general" }),
    );
    assert!(res_viol1.is_err(), "Cross-app invocation must trigger SandboxViolation");
    assert!(res_viol1.unwrap_err().contains("SandboxViolation"));

    // 3. Photos invoking community method
    let res_viol2 = DesktopPlatformManager::dispatch_webview_rpc(
        "photos",
        "community_createPost",
        serde_json::json!({ "title": "hello" }),
    );
    assert!(res_viol2.is_err());
    assert!(res_viol2.unwrap_err().contains("SandboxViolation"));
}
