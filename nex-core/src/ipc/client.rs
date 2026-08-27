use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use serde_json::Value;
use crate::ipc::rpc::{JsonRpcRequest, JsonRpcResponse};
use crate::object::types::ObjectID;

pub struct NexRpcClient {
    pub server_addr: String,
    pub req_id: AtomicU64,
}

impl NexRpcClient {
    pub fn new(server_addr: &str) -> Self {
        Self {
            server_addr: server_addr.to_string(),
            req_id: AtomicU64::new(1),
        }
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.req_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let mut stream = TcpStream::connect(&self.server_addr)
            .map_err(|e| format!("Failed to connect to Nex daemon at {}: {:?}", self.server_addr, e))?;

        let json_payload = serde_json::to_string(&req)
            .map_err(|e| format!("Serialization error: {:?}", e))?;

        stream.write_all(json_payload.as_bytes())
            .map_err(|e| format!("Socket write error: {:?}", e))?;
        stream.write_all(b"\n")
            .map_err(|e| format!("Socket write newline error: {:?}", e))?;
        stream.flush()
            .map_err(|e| format!("Socket flush error: {:?}", e))?;

        let mut reader = BufReader::new(stream);
        let mut resp_line = String::new();
        reader.read_line(&mut resp_line)
            .map_err(|e| format!("Socket read error: {:?}", e))?;

        let resp: JsonRpcResponse = serde_json::from_str(&resp_line)
            .map_err(|e| format!("Invalid JSON-RPC response from daemon: {:?}, raw line: '{}'", e, resp_line))?;

        if let Some(err) = resp.error {
            return Err(format!("RPC Error (code {}): {}", err.code, err.message));
        }

        resp.result.ok_or_else(|| "Empty RPC response result".to_string())
    }

    pub fn ping(&self) -> Result<String, String> {
        let res = self.call("nex_ping", serde_json::json!({}))?;
        let actor = res.get("actor_id").and_then(|v| v.as_str()).unwrap_or("");
        Ok(format!("PONG [Actor: {}]", actor))
    }

    pub fn get_status(&self) -> Result<Value, String> {
        self.call("nex_getStatus", serde_json::json!({}))
    }

    pub fn sync_now(&self) -> Result<Value, String> {
        self.call("nex_syncNow", serde_json::json!({}))
    }

    pub fn gc_cas(&self) -> Result<usize, String> {
        let res = self.call("nex_gcCas", serde_json::json!({}))?;
        let count = res.get("reclaimed_chunks").and_then(|v| v.as_u64()).unwrap_or(0);
        Ok(count as usize)
    }

    pub fn drive_put(&self, vpath: &str, content: &[u8]) -> Result<ObjectID, String> {
        let params = serde_json::json!({
            "path": vpath,
            "payload": hex::encode(content),
        });
        let res = self.call("nex_drivePut", params)?;
        let oid_hex = res.get("object_id").and_then(|v| v.as_str()).ok_or("Missing object_id in response")?;
        let bytes = hex::decode(oid_hex).map_err(|e| format!("Invalid object_id hex: {:?}", e))?;
        if bytes.len() != 32 {
            return Err("ObjectID must be 32 bytes".to_string());
        }
        let mut oid = [0u8; 32];
        oid.copy_from_slice(&bytes);
        Ok(oid)
    }

    pub fn drive_list(&self, vpath: &str) -> Result<Vec<Value>, String> {
        let params = serde_json::json!({ "path": vpath });
        let res = self.call("nex_driveList", params)?;
        let entries = res.get("entries").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        Ok(entries)
    }

    pub fn chat_send(&self, channel_id: [u8; 32], text: &str) -> Result<ObjectID, String> {
        let params = serde_json::json!({
            "channel_id": hex::encode(channel_id),
            "text": text,
        });
        let res = self.call("nex_chatSend", params)?;
        let oid_hex = res.get("object_id").and_then(|v| v.as_str()).ok_or("Missing object_id in response")?;
        let bytes = hex::decode(oid_hex).map_err(|e| format!("Invalid object_id hex: {:?}", e))?;
        let mut oid = [0u8; 32];
        oid.copy_from_slice(&bytes);
        Ok(oid)
    }

    pub fn community_post(&self, channel_id: [u8; 32], title: &str, content: &str) -> Result<ObjectID, String> {
        let params = serde_json::json!({
            "channel_id": hex::encode(channel_id),
            "title": title,
            "content": content,
        });
        let res = self.call("nex_communityPost", params)?;
        let oid_hex = res.get("object_id").and_then(|v| v.as_str()).ok_or("Missing object_id in response")?;
        let bytes = hex::decode(oid_hex).map_err(|e| format!("Invalid object_id hex: {:?}", e))?;
        let mut oid = [0u8; 32];
        oid.copy_from_slice(&bytes);
        Ok(oid)
    }
}
