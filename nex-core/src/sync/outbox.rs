use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NamespaceID};
use crate::identity::types::ActorID;
use crate::runtime::node::NexNode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub entry_id: [u8; 16],
    pub object_id: ObjectID,
    pub namespace: NamespaceID,
    pub target_peer: Option<ActorID>,
    pub payload_bytes: Vec<u8>,
    pub enqueued_epoch: u64,
    pub attempts: u32,
    pub acknowledged: bool,
}

pub struct OfflineOutboxStore {
    pub entries: BTreeMap<[u8; 16], OutboxEntry>,
}

impl OfflineOutboxStore {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn enqueue(&mut self, entry: OutboxEntry) {
        self.entries.insert(entry.entry_id, entry);
    }

    pub fn pending_entries(&self) -> Vec<OutboxEntry> {
        self.entries
            .values()
            .filter(|e| !e.acknowledged)
            .cloned()
            .collect()
    }

    pub fn acknowledge(&mut self, entry_id: &[u8; 16]) -> bool {
        if let Some(entry) = self.entries.get_mut(entry_id) {
            entry.acknowledged = true;
            true
        } else {
            false
        }
    }

    pub fn record_failure(&mut self, entry_id: &[u8; 16]) {
        if let Some(entry) = self.entries.get_mut(entry_id) {
            entry.attempts += 1;
        }
    }

    pub fn recover_from_node_state(&mut self, node: &NexNode) {
        // Reconstruct uncompacted mutations as outbox entries if not acknowledged
        for (obj_id, obj) in &node.state.object_store {
            let mut entry_id = [0u8; 16];
            entry_id[..8].copy_from_slice(&obj.created_epoch.to_le_bytes());
            entry_id[8..16].copy_from_slice(&obj_id[..8]);

            if !self.entries.contains_key(&entry_id) {
                self.entries.insert(entry_id, OutboxEntry {
                    entry_id,
                    object_id: *obj_id,
                    namespace: obj.namespace,
                    target_peer: None,
                    payload_bytes: obj.payload_bytes.clone(),
                    enqueued_epoch: obj.created_epoch,
                    attempts: 0,
                    acknowledged: false,
                });
            }
        }
    }
}
