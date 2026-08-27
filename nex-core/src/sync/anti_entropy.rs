use std::collections::{BTreeMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::model::{Mutation, MutationID, CrdtPayload};
use crate::hash::hash_mutation_body;
use crate::runtime::node::NexNode;
use crate::object::types::{ObjectType, NexObject};
use crate::sync::types::IngressDisposition;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncAdvertise {
    pub session_id: [u8; 16],
    pub current_epoch: u64,
    pub current_lamport: u64,
    pub latest_checkpoint_root: [u8; 32],
    pub frontier_mutation_ids: Vec<MutationID>,
    pub known_mutation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncDeltaRequest {
    pub session_id: [u8; 16],
    pub requested_mutations: Vec<MutationID>,
    pub max_batch_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStreamBatch {
    pub session_id: [u8; 16],
    pub batch_index: u32,
    pub total_batches: u32,
    pub mutations: Vec<Mutation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncBatchAck {
    pub session_id: [u8; 16],
    pub batch_index: u32,
    pub ingested_count: usize,
    pub remaining_window_credit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncComplete {
    pub session_id: [u8; 16],
    pub state_commitment: [u8; 32],
}

pub struct AntiEntropyEngine;

impl AntiEntropyEngine {
    pub fn generate_advertise(node: &mut NexNode, session_id: [u8; 16]) -> SyncAdvertise {
        let cp = node.state.state_node.compute_current_checkpoint();
        let frontier = node.state.state_node.frontier.iter().copied().collect();
        SyncAdvertise {
            session_id,
            current_epoch: node.state.current_epoch,
            current_lamport: node.state.state_node.current_lamport,
            latest_checkpoint_root: cp.body.state_root,
            frontier_mutation_ids: frontier,
            known_mutation_count: node.state.state_node.dag.len(),
        }
    }

    pub fn calculate_delta_request(node: &NexNode, adv: &SyncAdvertise) -> Option<SyncDeltaRequest> {
        let mut missing = Vec::new();
        for m_id in &adv.frontier_mutation_ids {
            if !node.state.state_node.dag.contains_key(m_id) {
                missing.push(*m_id);
            }
        }

        if !missing.is_empty() || node.state.state_node.dag.len() < adv.known_mutation_count {
            Some(SyncDeltaRequest {
                session_id: adv.session_id,
                requested_mutations: missing,
                max_batch_items: 64,
            })
        } else {
            None
        }
    }

    pub fn has_deltas_for_peer(node: &NexNode, adv: &SyncAdvertise) -> bool {
        node.state.state_node.dag.len() > adv.known_mutation_count
            || node.state.state_node.dag.keys().any(|k| !adv.frontier_mutation_ids.contains(k))
    }

    pub fn generate_batches_for_peer(
        node: &NexNode,
        session_id: [u8; 16],
        remote_known_frontier: &[MutationID],
        max_batch_items: usize,
    ) -> Vec<SyncStreamBatch> {
        // 1. Traverse causal ancestors of remote_known_frontier in local DAG
        let mut known_ancestors: HashSet<MutationID> = HashSet::new();
        let mut queue: Vec<MutationID> = remote_known_frontier.to_vec();

        while let Some(current_id) = queue.pop() {
            if known_ancestors.insert(current_id) {
                if let Some(m) = node.state.state_node.dag.get(&current_id) {
                    for p in &m.body.parents {
                        if !known_ancestors.contains(p) {
                            queue.push(*p);
                        }
                    }
                }
            }
        }

        // 2. Candidate mutations are those strictly outside known_ancestors
        let mut candidate_mutations: Vec<Mutation> = Vec::new();
        for (_id, mutation) in &node.state.state_node.dag {
            if !known_ancestors.contains(&mutation.id) {
                candidate_mutations.push(mutation.clone());
            }
        }

        candidate_mutations.sort_by(|a, b| {
            a.body.epoch.cmp(&b.body.epoch)
                .then(a.body.lamport.cmp(&b.body.lamport))
                .then(a.id.cmp(&b.id))
        });

        if candidate_mutations.is_empty() {
            return Vec::new();
        }

        let chunk_size = max_batch_items.max(1);
        let chunks: Vec<Vec<Mutation>> = candidate_mutations.chunks(chunk_size)
            .map(|c| c.to_vec())
            .collect();
        let total = chunks.len() as u32;

        chunks.into_iter().enumerate().map(|(idx, muts)| {
            SyncStreamBatch {
                session_id,
                batch_index: idx as u32,
                total_batches: total,
                mutations: muts,
            }
        }).collect()
    }

    pub fn generate_batches(node: &NexNode, req: &SyncDeltaRequest, remote_known_frontier: &[MutationID]) -> Vec<SyncStreamBatch> {
        Self::generate_batches_for_peer(node, req.session_id, remote_known_frontier, req.max_batch_items)
    }

    pub fn sync_object_store_entry(node: &mut NexNode, obj_id: &[u8; 32]) {
        if let Some((_opt_val, epoch, lamport, winning_id)) = node.state.state_node.crdt_state.get(obj_id) {
            if let Some(winning_mutation) = node.state.state_node.dag.get(winning_id) {
                match &winning_mutation.body.payload {
                    CrdtPayload::AddLWW { id, value } => {
                        if let Some(obj) = node.state.object_store.get_mut(id) {
                            obj.payload_bytes = value.clone();
                            obj.created_epoch = *epoch;
                            obj.created_lamport = *lamport;
                            obj.winning_mutation_id = *winning_id;
                            obj.tombstoned = false;
                        } else {
                            let obj = NexObject {
                                object_id: *id,
                                object_type: ObjectType::Synthetic(1),
                                namespace: [0u8; 32],
                                owner_actor_id: winning_mutation.body.author,
                                schema_version: 1,
                                created_epoch: *epoch,
                                created_lamport: *lamport,
                                winning_mutation_id: *winning_id,
                                metadata: BTreeMap::new(),
                                payload_bytes: value.clone(),
                                tombstoned: false,
                            };
                            node.state.object_store.insert(*id, obj);
                        }
                    }
                    CrdtPayload::Tombstone { id } | CrdtPayload::RemoveLWW { id } => {
                        if let Some(obj) = node.state.object_store.get_mut(id) {
                            obj.tombstoned = true;
                            obj.created_epoch = *epoch;
                            obj.created_lamport = *lamport;
                            obj.winning_mutation_id = *winning_id;
                        } else {
                            let obj = NexObject {
                                object_id: *id,
                                object_type: ObjectType::Synthetic(1),
                                namespace: [0u8; 32],
                                owner_actor_id: winning_mutation.body.author,
                                schema_version: 1,
                                created_epoch: *epoch,
                                created_lamport: *lamport,
                                winning_mutation_id: *winning_id,
                                metadata: BTreeMap::new(),
                                payload_bytes: Vec::new(),
                                tombstoned: true,
                            };
                            node.state.object_store.insert(*id, obj);
                        }
                    }
                }
            }
        }
    }

    pub fn ingest_batch(node: &mut NexNode, batch: SyncStreamBatch) -> Result<SyncBatchAck, String> {
        let mut ingested = 0;
        for m in batch.mutations {
            let expected_id = hash_mutation_body(&m.body);
            if m.id != expected_id {
                return Err(format!("Preflight verification failure: forged mutation id {:?}", m.id));
            }

            if !node.state.state_node.dag.contains_key(&m.id) {
                if let Some(wal) = &mut node.storage.wal {
                    let _ = wal.append_mutation(&m);
                }
                let (_disp, admitted_ids) = node.state.state_node.ingest_mutation_with_admissions(m.clone());
                node.state.latest_mutation_id = Some(m.id);

                if !admitted_ids.is_empty() {
                    for adm_id in admitted_ids {
                        if let Some(adm_mut) = node.state.state_node.dag.get(&adm_id) {
                            let target_obj_id = match &adm_mut.body.payload {
                                CrdtPayload::AddLWW { id, .. } | CrdtPayload::RemoveLWW { id } | CrdtPayload::Tombstone { id } => *id,
                            };
                            Self::sync_object_store_entry(node, &target_obj_id);
                        }
                    }
                }
                ingested += 1;
            }
        }

        Ok(SyncBatchAck {
            session_id: batch.session_id,
            batch_index: batch.batch_index,
            ingested_count: ingested,
            remaining_window_credit: 10,
        })
    }

    pub fn generate_complete(node: &mut NexNode, session_id: [u8; 16]) -> SyncComplete {
        let cp = node.state.state_node.compute_current_checkpoint();
        SyncComplete {
            session_id,
            state_commitment: cp.body.state_root,
        }
    }

    pub fn verify_convergence(node: &mut NexNode, comp: &SyncComplete) -> Result<bool, String> {
        let local_cp = node.state.state_node.compute_current_checkpoint();
        if local_cp.body.state_root == comp.state_commitment {
            Ok(true)
        } else {
            Err(format!(
                "Convergence mismatch: local {:?} != remote {:?}",
                local_cp.body.state_root, comp.state_commitment
            ))
        }
    }
}
