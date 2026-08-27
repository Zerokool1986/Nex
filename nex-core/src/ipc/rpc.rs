use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::runtime::production::{ProductionNodeSupervisor, NodeOperationalState};
use crate::runtime::node::NexNode;
use crate::api::NexAppApi;
use crate::object::types::ObjectType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub struct NexRpcDispatcher;

impl NexRpcDispatcher {
    pub fn dispatch(supervisor: &mut ProductionNodeSupervisor, req: JsonRpcRequest) -> JsonRpcResponse {
        if req.jsonrpc != "2.0" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Invalid Request: jsonrpc must be '2.0'".to_string(),
                }),
            };
        }

        match req.method.as_str() {
            "nex_getStatus" => {
                let status_val = serde_json::json!({
                    "state": match supervisor.state {
                        NodeOperationalState::Running => "RUNNING",
                        NodeOperationalState::ReplayingWal => "REPLAYING_WAL",
                        NodeOperationalState::Stopped => "STOPPED",
                        NodeOperationalState::Degraded => "DEGRADED",
                        NodeOperationalState::Uninitialized => "UNINITIALIZED",
                    },
                    "schema_version": supervisor.schema_version,
                    "cas_chunks_count": supervisor.cas.chunks.len(),
                    "panic_count": supervisor.panic_count,
                });
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(status_val),
                    error: None,
                }
            }
            "nex_gcCas" => {
                let empty_roots = std::collections::HashSet::new();
                let reclaimed = supervisor.gc_cas_unreachable(&empty_roots);
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({ "reclaimed_chunks": reclaimed })),
                    error: None,
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method '{}' not found", req.method),
                }),
            },
        }
    }

    pub fn dispatch_node(node: &mut NexNode, req: JsonRpcRequest) -> JsonRpcResponse {
        if req.jsonrpc != "2.0" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Invalid Request: jsonrpc must be '2.0'".to_string(),
                }),
            };
        }

        match req.method.as_str() {
            "nex_ping" => {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({
                        "status": "pong",
                        "actor_id": hex::encode(node.identity.actor_id),
                    })),
                    error: None,
                }
            }
            "nex_createObject" => {
                let ns_hex = req.params.get("namespace").and_then(|v| v.as_str()).unwrap_or("");
                let mut ns = [0u8; 32];
                if let Ok(b) = hex::decode(ns_hex) {
                    if b.len() == 32 {
                        ns.copy_from_slice(&b);
                    }
                }

                let payload_str = req.params.get("payload").and_then(|v| v.as_str()).unwrap_or("");
                let payload = hex::decode(payload_str).unwrap_or_else(|_| payload_str.as_bytes().to_vec());

                let mut metadata = BTreeMap::new();
                if let Some(obj) = req.params.get("metadata").and_then(|v| v.as_object()) {
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            metadata.insert(k.clone(), s.to_string());
                        }
                    }
                }

                match node.create_object(ns, ObjectType::Synthetic(1), metadata, payload) {
                    Ok(obj_id) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({ "object_id": hex::encode(obj_id) })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: format!("{:?}", e),
                        }),
                    },
                }
            }
            "nex_readObject" => {
                let obj_id_hex = req.params.get("object_id").and_then(|v| v.as_str()).unwrap_or("");
                let mut obj_id = [0u8; 32];
                if let Ok(b) = hex::decode(obj_id_hex) {
                    if b.len() == 32 {
                        obj_id.copy_from_slice(&b);
                    }
                }

                match node.read_object(&obj_id) {
                    Ok(obj) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({
                            "object_id": hex::encode(obj.object_id),
                            "payload_len": obj.payload_bytes.len(),
                            "tombstoned": obj.tombstoned,
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32001,
                            message: format!("{:?}", e),
                        }),
                    },
                }
            }
            "nex_syncNow" => {
                match node.sync_now() {
                    Ok(cp) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({
                            "state_root": hex::encode(cp.body.state_root),
                            "epoch": node.state.current_epoch,
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32002,
                            message: format!("{:?}", e),
                        }),
                    },
                }
            }
            "nex_getStatus" => {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({
                        "actor_id": hex::encode(node.identity.actor_id),
                        "objects_count": node.state.object_store.len(),
                        "dag_count": node.state.state_node.dag.len(),
                    })),
                    error: None,
                }
            }
            "nex_gcCas" => {
                let empty_roots = std::collections::HashSet::new();
                let reclaimed = node.gc_cas(&empty_roots);
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({ "reclaimed_chunks": reclaimed })),
                    error: None,
                }
            }
            "nex_drivePut" => {
                let raw_path = req.params.get("path").and_then(|v| v.as_str()).unwrap_or("/untitled.dat");
                let norm_path = crate::apps::drive::normalize_vpath(raw_path);
                let payload_str = req.params.get("payload").and_then(|v| v.as_str()).unwrap_or("");
                let payload = hex::decode(payload_str).unwrap_or_else(|_| payload_str.as_bytes().to_vec());

                let mut metadata = BTreeMap::new();
                metadata.insert("path".to_string(), norm_path.clone());
                metadata.insert("size".to_string(), format!("{}", payload.len()));

                let ns_drive = [0xD1; 32];
                match node.create_object(ns_drive, ObjectType::DriveInode, metadata, payload) {
                    Ok(obj_id) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({
                            "object_id": hex::encode(obj_id),
                            "path": norm_path,
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32010,
                            message: format!("{:?}", e),
                        }),
                    },
                }
            }
            "nex_driveList" => {
                let prefix = req.params.get("path").and_then(|v| v.as_str()).unwrap_or("/");
                let norm_prefix = crate::apps::drive::normalize_vpath(prefix);
                let ns_drive = [0xD1; 32];
                
                let mut entries = Vec::new();
                for (oid, obj) in &node.state.object_store {
                    if obj.namespace == ns_drive && !obj.tombstoned {
                        if let Some(p) = obj.metadata.get("path") {
                            if p.starts_with(&norm_prefix) || norm_prefix == "/" {
                                entries.push(serde_json::json!({
                                    "object_id": hex::encode(oid),
                                    "path": p,
                                    "size_bytes": obj.payload_bytes.len(),
                                    "created_epoch": obj.created_epoch,
                                }));
                            }
                        }
                    }
                }
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(serde_json::json!({ "entries": entries })),
                    error: None,
                }
            }
            "nex_chatSend" => {
                let chan_hex = req.params.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
                let text = req.params.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let ns_chat = [0xC1; 32];

                let mut metadata = BTreeMap::new();
                metadata.insert("channel".to_string(), chan_hex.to_string());
                metadata.insert("author".to_string(), hex::encode(node.identity.actor_id));

                match node.create_object(ns_chat, ObjectType::ChatMessage, metadata, text.as_bytes().to_vec()) {
                    Ok(obj_id) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({
                            "object_id": hex::encode(obj_id),
                            "channel_id": chan_hex,
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32020,
                            message: format!("{:?}", e),
                        }),
                    },
                }
            }
            "nex_communityPost" => {
                let chan_hex = req.params.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
                let title = req.params.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let content = req.params.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let ns_community = [0xB1; 32];

                let mut metadata = BTreeMap::new();
                metadata.insert("channel".to_string(), chan_hex.to_string());
                metadata.insert("title".to_string(), title.to_string());
                metadata.insert("author".to_string(), hex::encode(node.identity.actor_id));

                match node.create_object(ns_community, ObjectType::Community, metadata, content.as_bytes().to_vec()) {
                    Ok(obj_id) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({
                            "object_id": hex::encode(obj_id),
                            "title": title,
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32030,
                            message: format!("{:?}", e),
                        }),
                    },
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method '{}' not found", req.method),
                }),
            },
        }
    }
}

pub struct NexRpcServer {
    pub listener: TcpListener,
    pub node: Arc<Mutex<NexNode>>,
}

impl NexRpcServer {
    pub fn bind(addr: &str, node: Arc<Mutex<NexNode>>) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr)?;
        Ok(Self { listener, node })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr, std::io::Error> {
        self.listener.local_addr()
    }

    pub fn handle_client(stream: TcpStream, node: Arc<Mutex<NexNode>>) {
        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        });
        let mut writer = stream;

        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line) {
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                    let resp = {
                        let mut guard = match node.lock() {
                            Ok(g) => g,
                            Err(_) => return,
                        };
                        NexRpcDispatcher::dispatch_node(&mut *guard, req)
                    };
                    if let Ok(json_str) = serde_json::to_string(&resp) {
                        let _ = writer.write_all(json_str.as_bytes());
                        let _ = writer.write_all(b"\n");
                        let _ = writer.flush();
                    }
                }
            }
            line.clear();
        }
    }
}
