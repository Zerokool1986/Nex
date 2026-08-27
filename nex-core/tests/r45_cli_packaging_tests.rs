use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tempfile::tempdir;
use nex_core::cli::{NexCli, CliCommand};
use nex_core::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};
use nex_core::runtime::production::{ProductionNodeSupervisor, NodeOperationalState};

#[test]
fn test_r45_a_cli_argument_parsing() {
    // 1. Parse status command with custom data dir
    let args1 = vec!["--data-dir".to_string(), "/tmp/custom_nex".to_string(), "status".to_string()];
    let cmd1 = NexCli::parse_args(&args1);
    assert_eq!(cmd1, CliCommand::Status { data_dir: PathBuf::from("/tmp/custom_nex"), socket: None });

    // 2. Parse init command
    let args2 = vec!["init".to_string()];
    let cmd2 = NexCli::parse_args(&args2);
    assert_eq!(cmd2, CliCommand::Init { data_dir: PathBuf::from(".nex") });

    // 3. Parse unknown command
    let args3 = vec!["bogus_verb".to_string()];
    let cmd3 = NexCli::parse_args(&args3);
    assert_eq!(cmd3, CliCommand::Unknown("bogus_verb".to_string()));
}

#[test]
fn test_r45_b_ipc_json_rpc_dispatch() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut supervisor = ProductionNodeSupervisor::new(tmp.path(), key);
    supervisor.start().unwrap();

    // 1. Valid getStatus call
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 42,
        method: "nex_getStatus".to_string(),
        params: serde_json::Value::Null,
    };
    let resp = NexRpcDispatcher::dispatch(&mut supervisor, req);
    assert_eq!(resp.jsonrpc, "2.0");
    assert_eq!(resp.id, 42);
    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap()["state"], "RUNNING");

    // 2. Invalid RPC protocol version
    let bad_req = JsonRpcRequest {
        jsonrpc: "1.0".to_string(),
        id: 99,
        method: "nex_getStatus".to_string(),
        params: serde_json::Value::Null,
    };
    let bad_resp = NexRpcDispatcher::dispatch(&mut supervisor, bad_req);
    assert!(bad_resp.error.is_some());
    assert_eq!(bad_resp.error.unwrap().code, -32600);
}

#[test]
fn test_r45_c_e2e_cli_workflow() {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);

    let mut supervisor = ProductionNodeSupervisor::new(tmp.path(), key);

    // 1. Execute init
    let (code1, msg1) = NexCli::execute(CliCommand::Init { data_dir: tmp.path().to_path_buf() }, &mut supervisor);
    assert_eq!(code1, 0);
    assert!(msg1.contains("initialized"));

    // 2. Execute status
    let (code2, msg2) = NexCli::execute(CliCommand::Status { data_dir: tmp.path().to_path_buf(), socket: None }, &mut supervisor);
    assert_eq!(code2, 0);
    assert!(msg2.contains("RUNNING"));

    // 3. Execute GC
    let (code3, msg3) = NexCli::execute(CliCommand::GcCas { data_dir: tmp.path().to_path_buf(), socket: None }, &mut supervisor);
    assert_eq!(code3, 0);
    assert!(msg3.contains("reclaimed_chunks"));
}
