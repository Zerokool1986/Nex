use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use crate::runtime::production::ProductionNodeSupervisor;
use crate::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};
use crate::ipc::client::NexRpcClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Init { data_dir: PathBuf },
    Daemon { data_dir: PathBuf, port: u16 },
    Ping { socket: String },
    Status { data_dir: PathBuf, socket: Option<String> },
    Sync { socket: String },
    GcCas { data_dir: PathBuf, socket: Option<String> },
    DrivePut { socket: String, file_path: PathBuf, vpath: String },
    DriveList { socket: String, vpath: String },
    ChatSend { socket: String, channel_id: [u8; 32], message: String },
    CommunityPost { socket: String, channel_id: [u8; 32], title: String, content: String },
    Unknown(String),
}

pub struct NexCli;

impl NexCli {
    pub fn parse_args(args: &[String]) -> CliCommand {
        let mut data_dir = PathBuf::from(".nex");
        let mut socket = "127.0.0.1:44555".to_string();
        let mut explicit_socket = false;
        let mut port = 44555u16;

        let mut verb1 = String::new();
        let mut verb2 = String::new();
        let mut positional = Vec::new();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--data-dir" => {
                    if i + 1 < args.len() {
                        data_dir = PathBuf::from(&args[i + 1]);
                        i += 1;
                    }
                }
                "--socket" => {
                    if i + 1 < args.len() {
                        socket = args[i + 1].clone();
                        explicit_socket = true;
                        i += 1;
                    }
                }
                "--port" => {
                    if i + 1 < args.len() {
                        if let Ok(p) = args[i + 1].parse::<u16>() {
                            port = p;
                            socket = format!("127.0.0.1:{}", p);
                        }
                        i += 1;
                    }
                }
                other if !other.starts_with("--") => {
                    if verb1.is_empty() {
                        verb1 = other.to_string();
                    } else if verb2.is_empty() && (verb1 == "drive" || verb1 == "chat" || verb1 == "community" || verb1 == "daemon") {
                        verb2 = other.to_string();
                    } else {
                        positional.push(other.to_string());
                    }
                }
                _ => {}
            }
            i += 1;
        }

        match verb1.as_str() {
            "init" => CliCommand::Init { data_dir },
            "daemon" => CliCommand::Daemon { data_dir, port },
            "ping" => CliCommand::Ping { socket },
            "status" => CliCommand::Status {
                data_dir,
                socket: if explicit_socket { Some(socket) } else { None },
            },
            "sync" => CliCommand::Sync { socket },
            "gc" => CliCommand::GcCas {
                data_dir,
                socket: if explicit_socket { Some(socket) } else { None },
            },
            "drive" => match verb2.as_str() {
                "put" => {
                    let file_path = positional.get(0).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("data.txt"));
                    let vpath = positional.get(1).cloned().unwrap_or_else(|| format!("/{}", file_path.file_name().unwrap_or_default().to_string_lossy()));
                    CliCommand::DrivePut { socket, file_path, vpath }
                }
                "ls" | "list" => {
                    let vpath = positional.get(0).cloned().unwrap_or_else(|| "/".to_string());
                    CliCommand::DriveList { socket, vpath }
                }
                other => CliCommand::Unknown(format!("drive {}", other)),
            },
            "chat" => match verb2.as_str() {
                "send" => {
                    let chan_hex = positional.get(0).cloned().unwrap_or_default();
                    let mut channel_id = [0u8; 32];
                    if let Ok(b) = hex::decode(&chan_hex) {
                        if b.len() == 32 {
                            channel_id.copy_from_slice(&b);
                        }
                    }
                    let message = positional.get(1..).map(|s| s.join(" ")).unwrap_or_default();
                    CliCommand::ChatSend { socket, channel_id, message }
                }
                other => CliCommand::Unknown(format!("chat {}", other)),
            },
            "community" => match verb2.as_str() {
                "post" => {
                    let chan_hex = positional.get(0).cloned().unwrap_or_default();
                    let mut channel_id = [0u8; 32];
                    if let Ok(b) = hex::decode(&chan_hex) {
                        if b.len() == 32 {
                            channel_id.copy_from_slice(&b);
                        }
                    }
                    let title = positional.get(1).cloned().unwrap_or_else(|| "Untitled Post".to_string());
                    let content = positional.get(2..).map(|s| s.join(" ")).unwrap_or_default();
                    CliCommand::CommunityPost { socket, channel_id, title, content }
                }
                other => CliCommand::Unknown(format!("community {}", other)),
            },
            other => CliCommand::Unknown(other.to_string()),
        }
    }

    pub fn execute(cmd: CliCommand, supervisor: &mut ProductionNodeSupervisor) -> (i32, String) {
        match cmd {
            CliCommand::Init { .. } => {
                let _ = supervisor.start();
                (0, "Node successfully initialized".to_string())
            }
            CliCommand::Status { .. } => {
                let req = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    id: 1,
                    method: "nex_getStatus".to_string(),
                    params: serde_json::Value::Null,
                };
                let resp = NexRpcDispatcher::dispatch(supervisor, req);
                (0, serde_json::to_string_pretty(&resp).unwrap_or_default())
            }
            CliCommand::GcCas { .. } => {
                let req = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    id: 2,
                    method: "nex_gcCas".to_string(),
                    params: serde_json::Value::Null,
                };
                let resp = NexRpcDispatcher::dispatch(supervisor, req);
                (0, serde_json::to_string_pretty(&resp).unwrap_or_default())
            }
            other => (1, format!("Command '{:?}' requires running daemon connection", other)),
        }
    }

    pub fn execute_client(cmd: &CliCommand, client: &NexRpcClient) -> (i32, String) {
        match cmd {
            CliCommand::Ping { .. } => match client.ping() {
                Ok(msg) => (0, msg),
                Err(e) => (1, format!("Error: {}", e)),
            },
            CliCommand::Status { .. } => match client.get_status() {
                Ok(val) => (0, serde_json::to_string_pretty(&val).unwrap_or_default()),
                Err(e) => (1, format!("Error: {}", e)),
            },
            CliCommand::Sync { .. } => match client.sync_now() {
                Ok(val) => (0, serde_json::to_string_pretty(&val).unwrap_or_default()),
                Err(e) => (1, format!("Error: {}", e)),
            },
            CliCommand::GcCas { .. } => match client.gc_cas() {
                Ok(count) => (0, format!("Reclaimed {} unreachable CAS chunks", count)),
                Err(e) => (1, format!("Error: {}", e)),
            },
            CliCommand::DrivePut { file_path, vpath, .. } => {
                let content = match fs::read(file_path) {
                    Ok(b) => b,
                    Err(e) => return (1, format!("Failed to read local file '{:?}': {:?}", file_path, e)),
                };
                match client.drive_put(vpath, &content) {
                    Ok(oid) => (0, format!("File uploaded to Drive: {} (ObjectID: {})", vpath, hex::encode(oid))),
                    Err(e) => (1, format!("Drive upload failed: {}", e)),
                }
            }
            CliCommand::DriveList { vpath, .. } => match client.drive_list(vpath) {
                Ok(entries) => (0, serde_json::to_string_pretty(&entries).unwrap_or_default()),
                Err(e) => (1, format!("Drive list failed: {}", e)),
            },
            CliCommand::ChatSend { channel_id, message, .. } => match client.chat_send(*channel_id, message) {
                Ok(oid) => (0, format!("Message sent (ObjectID: {})", hex::encode(oid))),
                Err(e) => (1, format!("Chat send failed: {}", e)),
            },
            CliCommand::CommunityPost { channel_id, title, content, .. } => {
                match client.community_post(*channel_id, title, content) {
                    Ok(oid) => (0, format!("Post published: '{}' (ObjectID: {})", title, hex::encode(oid))),
                    Err(e) => (1, format!("Community post failed: {}", e)),
                }
            }
            CliCommand::Unknown(u) => (1, format!("Unknown command: '{}'", u)),
            other => (1, format!("Unsupported client operation: {:?}", other)),
        }
    }
}
