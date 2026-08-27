use std::collections::BTreeMap;
use super::types::{OperationBody, RegisterState, OperationIndex};
use super::topology::{Epoch, LamportRank};
use crate::HashRef;

#[derive(Debug, Clone)]
pub struct EvaluationItem {
    pub epoch: Epoch,
    pub lamport_rank: LamportRank,
    pub mutation_id: HashRef,
    pub operation_index: OperationIndex,
    pub body: OperationBody,
    pub op_tag: HashRef,
}

impl PartialEq for EvaluationItem {
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch &&
        self.lamport_rank == other.lamport_rank &&
        self.mutation_id == other.mutation_id &&
        self.operation_index == other.operation_index
    }
}

impl Eq for EvaluationItem {}

impl PartialOrd for EvaluationItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EvaluationItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.epoch.cmp(&other.epoch)
            .then_with(|| self.lamport_rank.cmp(&other.lamport_rank))
            .then_with(|| self.mutation_id.cmp(&other.mutation_id))
            .then_with(|| self.operation_index.cmp(&other.operation_index))
    }
}

pub struct CrdtEvaluator {
    pub register_state: BTreeMap<Vec<u8>, RegisterState>,
}

impl Default for CrdtEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl CrdtEvaluator {
    pub fn new() -> Self {
        Self { register_state: BTreeMap::new() }
    }

    pub fn evaluate(&mut self, items: Vec<EvaluationItem>) {
        let mut deduplicated = items;
        deduplicated.sort();
        deduplicated.dedup(); // Replay Idempotency via strict equality
        
        let mut current_epoch = Epoch(0);
        
        for item in deduplicated {
            if item.epoch > current_epoch {
                self.register_state.clear();
                current_epoch = item.epoch;
            }
            
            match item.body {
                OperationBody::Add { key, payload } => {
                    self.register_state.insert(key, RegisterState::Add {
                        op_tag: item.op_tag,
                        payload,
                        epoch: item.epoch,
                        lamport_rank: item.lamport_rank,
                        mutation_id: item.mutation_id,
                        operation_index: item.operation_index,
                    });
                }
                OperationBody::Remove { key } => {
                    self.register_state.insert(key, RegisterState::Remove {
                        op_tag: item.op_tag,
                        epoch: item.epoch,
                        lamport_rank: item.lamport_rank,
                        mutation_id: item.mutation_id,
                        operation_index: item.operation_index,
                    });
                }
                OperationBody::InitObject | OperationBody::Resurrect => {
                    // Control operations, no register write.
                }
            }
        }
    }
}
