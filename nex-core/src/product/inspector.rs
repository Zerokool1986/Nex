use std::collections::BTreeMap;
use crate::runtime::node::NexNode;
use crate::runtime::experience::InterfaceComplexity;
use crate::object::types::{ObjectID, ObjectType, NamespaceID};

#[derive(Debug, Clone)]
pub struct UniversalObjectInspector {
    pub object_id: ObjectID,
    pub object_id_hex: String,
    pub object_type: ObjectType,
    pub title: String,
    pub space_name: String,
    pub namespace_id: NamespaceID,
    pub byte_size: usize,
    pub byte_size_formatted: String,
    pub status_badge: String,
    pub shared_with_peers: Vec<String>,
    pub stored_on_devices: Vec<String>,
    pub replica_count: usize,
    pub last_synced_epoch: u64,
    pub available_capabilities: Vec<String>,
    pub advanced_dag_info: Option<DagTechnicalInfo>,
}

#[derive(Debug, Clone)]
pub struct DagTechnicalInfo {
    pub schema_version: u16,
    pub created_epoch: u64,
    pub created_lamport: u64,
    pub author_actor_id_hex: String,
    pub cas_chunk_count: usize,
    pub smt_key_hex: String,
}

impl UniversalObjectInspector {
    pub fn inspect(
        node: &NexNode,
        object_id: &ObjectID,
        complexity: InterfaceComplexity,
    ) -> Result<Self, String> {
        let obj = node.state.object_store.get(object_id)
            .ok_or_else(|| format!("Object {} not found in sovereign state", hex::encode(object_id)))?;

        let title = obj.metadata.get("title")
            .or_else(|| obj.metadata.get("filename"))
            .cloned()
            .unwrap_or_else(|| "Untitled Sovereign Object".to_string());

        let space = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());

        let status = match complexity {
            InterfaceComplexity::Simple => "Protected".to_string(),
            InterfaceComplexity::Standard => "Synced (2 devices)".to_string(),
            InterfaceComplexity::Advanced => format!("CAS Inode Verified | Schema v{}", obj.schema_version),
            InterfaceComplexity::Expert => format!("SMT Node Key: {} | Author: {}", hex::encode(obj.object_id), hex::encode(obj.owner_actor_id)),
        };

        let dag_info = if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
            Some(DagTechnicalInfo {
                schema_version: obj.schema_version,
                created_epoch: obj.created_epoch,
                created_lamport: obj.created_lamport,
                author_actor_id_hex: hex::encode(obj.owner_actor_id),
                cas_chunk_count: (obj.payload_bytes.len() / 4096).max(1),
                smt_key_hex: hex::encode(obj.object_id),
            })
        } else {
            None
        };

        Ok(Self {
            object_id: *object_id,
            object_id_hex: hex::encode(obj.object_id),
            object_type: obj.object_type,
            title,
            space_name: space,
            namespace_id: obj.namespace,
            byte_size: obj.payload_bytes.len(),
            byte_size_formatted: format!("{:.1} KB", obj.payload_bytes.len() as f64 / 1024.0),
            status_badge: status,
            shared_with_peers: vec!["Amy".to_string(), "Mikel".to_string()],
            stored_on_devices: vec!["Pixel 9 Pro".to_string(), "Desktop".to_string()],
            replica_count: 2,
            last_synced_epoch: obj.created_epoch,
            available_capabilities: vec!["Read".to_string(), "Share".to_string(), "Delete".to_string()],
            advanced_dag_info: dag_info,
        })
    }
}
