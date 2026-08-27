use serde::{Deserialize, Serialize};
use crate::model::{Mutation, MutationID, Checkpoint, CheckpointID, Boundary};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IngressDisposition {
    Invalid(String),
    Duplicate(MutationID),
    DependencyGap { missing_parents: Vec<MutationID> },
    Rejected(String),
    AdmittedApplied(MutationID),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusAnnouncement {
    pub node_id: String,
    pub latest_checkpoint_id: CheckpointID,
    pub frontier: Vec<MutationID>,
    pub boundary: Boundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyRequest {
    pub requester_node_id: String,
    pub requested_ids: Vec<MutationID>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyResponse {
    pub responder_node_id: String,
    pub mutations: Vec<Mutation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationRequest {
    pub requester_node_id: String,
    pub local_frontier: Vec<MutationID>,
    pub known_checkpoint_id: Option<CheckpointID>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastSyncBundle {
    pub sender_node_id: String,
    pub checkpoint: Checkpoint,
    pub proof_bytes: Vec<u8>,
    pub image_id: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMessage {
    StatusAnnouncement(StatusAnnouncement),
    DependencyRequest(DependencyRequest),
    DependencyResponse(DependencyResponse),
    ReconciliationRequest(ReconciliationRequest),
    FastSyncBundle(FastSyncBundle),
    DirectMutationBroadcast(Mutation),
}
