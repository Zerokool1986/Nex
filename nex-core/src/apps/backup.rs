use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NamespaceID, ObjectType};
use crate::api::NexAppApi;
use crate::apps::drive::CasChunkStore;
use crate::identity::types::CapabilityProof;

pub const DOMAIN_BACKUP_SNAPSHOT: &[u8] = b"NEX/BACKUP/SNAPSHOT/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSnapshot {
    pub snapshot_id: ObjectID,
    pub namespace: NamespaceID,
    pub label: String,
    pub content_root: [u8; 32],
    pub chunk_digests: Vec<[u8; 32]>,
    pub total_byte_size: u64,
    pub created_epoch: u64,
}

pub struct NexBackupEngine<A: NexAppApi> {
    pub namespace_id: NamespaceID,
    pub api: A,
    pub cas: CasChunkStore,
    pub snapshots: BTreeMap<ObjectID, BackupSnapshot>,
}

impl<A: NexAppApi> NexBackupEngine<A> {
    pub fn new(namespace_id: NamespaceID, api: A, cas: CasChunkStore) -> Self {
        Self {
            namespace_id,
            api,
            cas,
            snapshots: BTreeMap::new(),
        }
    }

    pub fn create_backup(
        &mut self,
        label: &str,
        data: &[u8],
        epoch: u64,
        proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let (content_root, chunk_digests) = self.cas.store_file(data);

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_BACKUP_SNAPSHOT);
        hasher.update(&self.namespace_id);
        hasher.update(label.as_bytes());
        hasher.update(&content_root);
        hasher.update(&epoch.to_le_bytes());
        let snapshot = BackupSnapshot {
            snapshot_id: [0u8; 32],
            namespace: self.namespace_id,
            label: label.to_string(),
            content_root,
            chunk_digests,
            total_byte_size: data.len() as u64,
            created_epoch: epoch,
        };

        let encoded = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
        let mut meta = BTreeMap::new();
        meta.insert("label".to_string(), label.to_string());
        meta.insert("size_bytes".to_string(), format!("{}", data.len()));

        let _ = proof;
        let obj_id = self.api.create_object(
            self.namespace_id,
            ObjectType::BackupIndex,
            meta,
            encoded,
        ).map_err(|e| format!("{:?}", e))?;

        let mut final_snap = snapshot;
        final_snap.snapshot_id = obj_id;
        self.snapshots.insert(obj_id, final_snap);
        Ok(obj_id)
    }

    pub fn restore_backup(&self, snapshot_id: &ObjectID) -> Result<Vec<u8>, String> {
        let obj = self.api.read_object(snapshot_id).map_err(|e| format!("{:?}", e))?;
        if obj.tombstoned {
            return Err("Backup snapshot is tombstoned".into());
        }
        let snapshot: BackupSnapshot = serde_json::from_slice(&obj.payload_bytes).map_err(|e| e.to_string())?;
        self.cas.assemble_file(&snapshot.chunk_digests)
    }

    pub fn prune_backup(&mut self, snapshot_id: ObjectID, proof: Option<CapabilityProof>) -> Result<(), String> {
        self.api.delete_object(snapshot_id, proof).map_err(|e| format!("{:?}", e))?;
        self.snapshots.remove(&snapshot_id);
        Ok(())
    }

    pub fn list_backups(&self) -> Vec<BackupSnapshot> {
        self.snapshots.values().cloned().collect()
    }
}
