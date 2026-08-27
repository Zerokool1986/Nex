use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::apps::platform::{NexUri, NexUriResolver};
use crate::runtime::node::NexNode;
use crate::identity::types::{CapabilityProof, OP_READ};
use crate::identity::verifier::verify_capability_chain;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

pub struct NexWebGateway;

impl NexWebGateway {
    pub fn handle_http_get(
        node: &NexNode,
        uri_path: &str,
        headers: &BTreeMap<String, String>,
    ) -> HttpResponse {
        // Expected uri_path: "/nex/<actor_hex>/<ns_hex>/<path>"
        if !uri_path.starts_with("/nex/") {
            return HttpResponse {
                status_code: 404,
                headers: BTreeMap::new(),
                body: b"Not Found: Path must begin with /nex/".to_vec(),
            };
        }

        let nex_uri_str = format!("nex://{}", &uri_path[5..]);
        let uri = match NexUri::parse(&nex_uri_str) {
            Ok(u) => u,
            Err(e) => {
                return HttpResponse {
                    status_code: 400,
                    headers: BTreeMap::new(),
                    body: format!("Bad Request: {}", e).into_bytes(),
                };
            }
        };

        // Capability authorization check if header is present
        if let Some(proof_hex) = headers.get("x-nex-capability-proof") {
            if let Ok(proof_bytes) = hex::decode(proof_hex) {
                if let Ok(proof) = serde_json::from_slice::<CapabilityProof>(&proof_bytes) {
                    let revocations = BTreeMap::new();
                    let root_actor = uri.actor_id;
                    if verify_capability_chain(
                        &proof,
                        OP_READ,
                        &uri.namespace,
                        None,
                        0,
                        &revocations,
                        &root_actor,
                    ).is_err() {
                        return HttpResponse {
                            status_code: 403,
                            headers: BTreeMap::new(),
                            body: b"Forbidden: Invalid Capability Proof".to_vec(),
                        };
                    }
                }
            }
        }

        // Resolve SMT Object
        match NexUriResolver::resolve_uri(node, &uri) {
            Some(obj) => {
                let mut resp_headers = BTreeMap::new();
                let mime = obj.metadata.get("content-type")
                    .cloned()
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                resp_headers.insert("content-type".to_string(), mime);
                resp_headers.insert("content-length".to_string(), obj.payload_bytes.len().to_string());

                let etag = hex::encode(Sha256::digest(&obj.payload_bytes));
                resp_headers.insert("etag".to_string(), format!("\"{}\"", etag));
                resp_headers.insert("cache-control".to_string(), "public, max-age=3600, immutable".to_string());

                HttpResponse {
                    status_code: 200,
                    headers: resp_headers,
                    body: obj.payload_bytes,
                }
            }
            None => HttpResponse {
                status_code: 404,
                headers: BTreeMap::new(),
                body: b"Not Found: Object does not exist".to_vec(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebAppManifest {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub entrypoint: String,
    pub content_security_policy: String,
}

impl WebAppManifest {
    pub fn default_secure(app_id: &str, name: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            entrypoint: "/index.html".to_string(),
            content_security_policy: "default-src 'self' nex:; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline';".to_string(),
        }
    }
}

pub struct WebRtcNaspBridge;

impl WebRtcNaspBridge {
    pub fn frame_data_channel_message(payload: &[u8]) -> Vec<u8> {
        let mut framed = Vec::with_capacity(4 + payload.len());
        framed.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        framed.extend_from_slice(payload);
        framed
    }

    pub fn unframe_data_channel_message(data: &[u8]) -> Result<&[u8], String> {
        if data.len() < 4 {
            return Err("Incomplete WebRTC frame: under 4 bytes".into());
        }
        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + len {
            return Err("Incomplete WebRTC frame payload".into());
        }
        Ok(&data[4..4 + len])
    }
}
