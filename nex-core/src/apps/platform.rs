use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use ed25519_dalek::SigningKey;
use crate::model::ActorID;
use crate::object::types::{NamespaceID, ObjectID, ObjectType, NexObject};
use crate::runtime::node::NexNode;
use crate::identity::types::{CapabilityProof, CapabilityToken, OP_ALL, OP_READ, OP_WRITE};
use crate::identity::verifier::derive_actor_id;
use crate::api::NexAppApi;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NexUri {
    pub raw: String,
    pub actor_id: ActorID,
    pub namespace: NamespaceID,
    pub path: String,
}

impl NexUri {
    pub fn parse(uri_str: &str) -> Result<Self, String> {
        if !uri_str.starts_with("nex://") {
            return Err("Invalid URI scheme: must start with nex://".into());
        }
        let rest = &uri_str[6..];
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() < 2 {
            return Err("URI format must be nex://<actor_id>/<namespace>[/<path>]".into());
        }

        let actor_bytes = hex::decode(parts[0]).map_err(|e| format!("Invalid actor_id hex: {:?}", e))?;
        if actor_bytes.len() != 32 {
            return Err("actor_id must be 32 bytes".into());
        }
        let mut actor_id = [0u8; 32];
        actor_id.copy_from_slice(&actor_bytes);

        let ns_bytes = hex::decode(parts[1]).map_err(|e| format!("Invalid namespace hex: {:?}", e))?;
        if ns_bytes.len() != 32 {
            return Err("namespace must be 32 bytes".into());
        }
        let mut namespace = [0u8; 32];
        namespace.copy_from_slice(&ns_bytes);

        let path = if parts.len() == 3 {
            format!("/{}", parts[2])
        } else {
            "/".to_string()
        };

        Ok(Self {
            raw: uri_str.to_string(),
            actor_id,
            namespace,
            path,
        })
    }
}

pub struct NexUriResolver;

impl NexUriResolver {
    pub fn resolve_uri(node: &NexNode, uri: &NexUri) -> Option<NexObject> {
        for (_oid, obj) in &node.state.object_store {
            if obj.namespace == uri.namespace && !obj.tombstoned {
                if let Some(p) = obj.metadata.get("path") {
                    if p == &uri.path {
                        return Some(obj.clone());
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialGeoPoint {
    pub lat: f64,
    pub lon: f64,
    pub geohash: String,
}

impl SpatialGeoPoint {
    pub fn new(lat: f64, lon: f64) -> Self {
        let geohash = format!("{:08x}", ((lat + 90.0) * 10000.0) as u32 ^ (((lon + 180.0) * 10000.0) as u32));
        Self { lat, lon, geohash }
    }
}

pub struct SpatialMapEngine;

impl SpatialMapEngine {
    pub fn query_bounding_box(
        node: &NexNode,
        min_lat: f64,
        max_lat: f64,
        min_lon: f64,
        max_lon: f64,
    ) -> Vec<NexObject> {
        let ns_maps = [0xAA; 32];
        let mut results = Vec::new();

        for (_oid, obj) in &node.state.object_store {
            if obj.namespace == ns_maps && !obj.tombstoned {
                if let (Some(lat_str), Some(lon_str)) = (obj.metadata.get("lat"), obj.metadata.get("lon")) {
                    if let (Ok(lat), Ok(lon)) = (lat_str.parse::<f64>(), lon_str.parse::<f64>()) {
                        if lat >= min_lat && lat <= max_lat && lon >= min_lon && lon <= max_lon {
                            results.push(obj.clone());
                        }
                    }
                }
            }
        }
        results
    }
}

pub struct GroupFederationEngine;

impl GroupFederationEngine {
    pub fn create_group_capability_token(
        group_root: &SigningKey,
        member_actor: ActorID,
        group_id: [u8; 32],
        allowed_ops: u32,
    ) -> CapabilityProof {
        let root_pub = group_root.verifying_key().to_bytes().to_vec();
        let issuer = derive_actor_id(crate::identity::types::KeyType::Ed25519, &root_pub);

        let token = CapabilityToken {
            issuer,
            subject: member_actor,
            namespace: group_id,
            object_id: None,
            allowed_operations: allowed_ops,
            delegation_depth: 1,
            not_before_epoch: 0,
            expires_at_epoch: 10000,
            parent_token_hash: None,
        };

        use ed25519_dalek::Signer;
        let token_hash = crate::identity::verifier::hash_capability_token(&token);
        let signature = group_root.sign(&token_hash).to_bytes().to_vec();

        CapabilityProof {
            token,
            issuer_pubkey: Some(root_pub),
            parent_proof: None,
            signature,
        }
    }
}

pub struct PetnameDirectory {
    pub aliases: BTreeMap<String, ActorID>,
}

impl PetnameDirectory {
    pub fn new() -> Self {
        Self {
            aliases: BTreeMap::new(),
        }
    }

    pub fn set_petname(&mut self, alias: &str, actor: ActorID) {
        self.aliases.insert(alias.to_lowercase(), actor);
    }

    pub fn resolve_petname(&self, alias: &str) -> Option<ActorID> {
        self.aliases.get(&alias.to_lowercase()).cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedMutation {
    pub id: u64,
    pub namespace: NamespaceID,
    pub obj_type: ObjectType,
    pub metadata: BTreeMap<String, String>,
    pub payload: Vec<u8>,
}

pub struct OfflineOutbox {
    pub queue: Vec<QueuedMutation>,
    pub next_id: u64,
}

impl OfflineOutbox {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            next_id: 1,
        }
    }

    pub fn enqueue(
        &mut self,
        namespace: NamespaceID,
        obj_type: ObjectType,
        metadata: BTreeMap<String, String>,
        payload: Vec<u8>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push(QueuedMutation {
            id,
            namespace,
            obj_type,
            metadata,
            payload,
        });
        id
    }

    pub fn flush_to_node(&mut self, node: &mut NexNode) -> Result<usize, String> {
        let count = self.queue.len();
        for item in self.queue.drain(..) {
            node.create_object(item.namespace, item.obj_type, item.metadata, item.payload)
                .map_err(|e| format!("{:?}", e))?;
        }
        Ok(count)
    }
}
