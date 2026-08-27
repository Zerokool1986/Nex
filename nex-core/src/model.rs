use serde::{Serialize, Deserialize};
use std::ops::Deref;

pub type CheckpointID = [u8; 32];
pub type MutationID = [u8; 32];
pub type Identity = [u8; 32];
pub type ActorID = [u8; 32];
pub type StateCommitment = [u8; 32];
pub type AccumulatorRoot = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionState {
    Unknown = 0,
    Admitted = 1,
    HistoricalInvalid = 2,
    Absent = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtPayload {
    AddLWW { id: [u8; 32], value: Vec<u8> },
    RemoveLWW { id: [u8; 32] },
    Tombstone { id: [u8; 32] },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationBody {
    pub author: Identity,
    pub parents: Vec<MutationID>,
    pub lamport: u64,
    pub epoch: u64,
    pub is_resurrect: bool,
    pub payload: CrdtPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mutation {
    pub id: MutationID,
    pub body: MutationBody,
}

impl Mutation {
    pub fn new(id: MutationID, body: MutationBody) -> Self {
        Self { id, body }
    }
}

impl Deref for Mutation {
    type Target = MutationBody;
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Boundary {
    pub max_epoch: u64,
    pub max_lamport: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointBody {
    pub state_root: [u8; 32],
    pub causal_root: [u8; 32],
    pub admission_root: [u8; 32],
    pub frontier: Vec<MutationID>,
    pub boundary: Boundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointID,
    pub body: CheckpointBody,
}

impl Checkpoint {
    pub fn new(id: CheckpointID, body: CheckpointBody) -> Self {
        Self { id, body }
    }
}

impl Deref for Checkpoint {
    type Target = CheckpointBody;
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateEncoding {
    pub mutation_id: MutationID,
    pub lamport: u64,
    pub epoch: u64,
    pub is_resurrect: bool,
    pub payload: CrdtPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicStatement {
    pub semantic_abi_version: u32,
    pub input_commitment: [u8; 32],
    pub frontier_commitment: [u8; 32],
    pub claimed_state_root: [u8; 32],
    pub claimed_causal_root: [u8; 32],
    pub claimed_admission_root: [u8; 32],
    pub claimed_boundary: Boundary,
    pub claimed_checkpoint_id: CheckpointID,
    pub initial_smt_root: [u8; 32],
    pub final_smt_root: [u8; 32],
    pub mutations_admitted: u32,
}
