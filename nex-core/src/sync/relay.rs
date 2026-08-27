use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::identity::types::ActorID;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayEnvelope {
    pub relay_session_id: [u8; 16],
    pub sender_actor: ActorID,
    pub recipient_actor: ActorID,
    pub encrypted_payload: Vec<u8>,
    pub ephemeral_nonce: [u8; 16],
    pub expiration_epoch: u64,
}

pub struct RelayStore {
    pub buffered_envelopes: BTreeMap<ActorID, Vec<RelayEnvelope>>,
}

impl RelayStore {
    pub fn new() -> Self {
        Self {
            buffered_envelopes: BTreeMap::new(),
        }
    }

    pub fn buffer_envelope(&mut self, envelope: RelayEnvelope) {
        self.buffered_envelopes
            .entry(envelope.recipient_actor)
            .or_default()
            .push(envelope);
    }

    pub fn drain_for_recipient(&mut self, recipient: &ActorID, current_epoch: u64) -> Vec<RelayEnvelope> {
        if let Some(list) = self.buffered_envelopes.remove(recipient) {
            list.into_iter()
                .filter(|env| current_epoch <= env.expiration_epoch)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn total_buffered_count(&self) -> usize {
        self.buffered_envelopes.values().map(|v| v.len()).sum()
    }
}
