use crate::HashRef;
use serde::{Serialize, Deserialize};
use super::topology::{Epoch, LamportRank};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OperationIndex(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationBody {
    InitObject,
    Add { key: Vec<u8>, payload: Vec<u8> },
    Remove { key: Vec<u8> },
    Resurrect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegisterState {
    Add {
        op_tag: HashRef,
        payload: Vec<u8>,
        epoch: Epoch,
        lamport_rank: LamportRank,
        mutation_id: HashRef,
        operation_index: OperationIndex,
    },
    Remove {
        op_tag: HashRef,
        epoch: Epoch,
        lamport_rank: LamportRank,
        mutation_id: HashRef,
        operation_index: OperationIndex,
    },
}
