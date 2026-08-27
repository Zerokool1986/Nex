use std::collections::{BTreeMap, BTreeSet, HashSet};
use crate::model::{
    Mutation, MutationID, Checkpoint, CheckpointBody, Boundary, CrdtPayload
};
use crate::hash::{
    hash_mutation_body, hash_checkpoint_body, hash_canonical,
    DOMAIN_STATE_ROOT, DOMAIN_CAUSAL_ROOT, DOMAIN_ADMISSION_ROOT
};
use crate::sync::types::IngressDisposition;

#[derive(Debug, Clone)]
pub struct VirtualNode {
    pub node_id: String,
    pub dag: BTreeMap<MutationID, Mutation>,
    pub crdt_state: BTreeMap<[u8; 32], (Option<Vec<u8>>, u64, u64, MutationID)>,
    pub dependency_buffer: BTreeMap<MutationID, (Mutation, HashSet<MutationID>)>,
    pub frontier: BTreeSet<MutationID>,
    pub latest_checkpoint: Option<Checkpoint>,
    pub current_lamport: u64,
}

impl VirtualNode {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            dag: BTreeMap::new(),
            crdt_state: BTreeMap::new(),
            dependency_buffer: BTreeMap::new(),
            frontier: BTreeSet::new(),
            latest_checkpoint: None,
            current_lamport: 0,
        }
    }

    /// Ingests an incoming mutation through the strict 6-state mutation lifecycle
    pub fn ingest_mutation(&mut self, mutation: Mutation) -> IngressDisposition {
        // --- 1. CRYPTOGRAPHIC PREIMAGE & VALIDATION ---
        let computed_id = match crate::hash::try_hash_mutation_body(&mutation.body) {
            Ok(id) => id,
            Err(e) => return IngressDisposition::Invalid(format!("Canonical serialization error: {:?}", e)),
        };
        if mutation.id != computed_id {
            return IngressDisposition::Invalid("Forged MutationID: ID != hash(Body)".into());
        }

        // --- 2. LOCAL KNOWLEDGE LOOKUP (DUPLICATE CHECK) ---
        if self.dag.contains_key(&mutation.id) {
            return IngressDisposition::Duplicate(mutation.id);
        }

        // --- 3. DEPENDENCY CHECK (MISSING PARENTS) ---
        let mut missing_parents = Vec::new();
        for p in &mutation.body.parents {
            if !self.dag.contains_key(p) {
                missing_parents.push(*p);
            }
        }

        if !missing_parents.is_empty() {
            let missing_set: HashSet<MutationID> = missing_parents.iter().copied().collect();
            self.dependency_buffer.insert(mutation.id, (mutation, missing_set));
            return IngressDisposition::DependencyGap { missing_parents };
        }

        // --- 4. CAUSAL ADMISSIBILITY VALIDATION ---
        for i in 1..mutation.body.parents.len() {
            if mutation.body.parents[i - 1] >= mutation.body.parents[i] {
                return IngressDisposition::Rejected("Parents not strictly sorted without duplicates".into());
            }
        }

        if mutation.body.parents.is_empty() {
            if mutation.body.lamport != 0 || mutation.body.epoch != 0 {
                return IngressDisposition::Rejected("Genesis mutation must have Lamport 0 and Epoch 0".into());
            }
        } else {
            if mutation.body.lamport == 0 {
                return IngressDisposition::Rejected("Non-genesis mutation cannot have Lamport 0".into());
            }

            let mut max_p_lamp = 0u64;
            let mut max_p_epoch = 0u64;

            for p_id in &mutation.body.parents {
                let parent = self.dag.get(p_id).expect("Parent must be present in DAG");
                if parent.body.lamport > max_p_lamp {
                    max_p_lamp = parent.body.lamport;
                }
                if parent.body.epoch > max_p_epoch {
                    max_p_epoch = parent.body.epoch;
                }
            }

            let expected_lamp = max_p_lamp + 1;
            let expected_epoch = if mutation.body.is_resurrect { max_p_epoch + 1 } else { max_p_epoch };

            if mutation.body.lamport != expected_lamp {
                return IngressDisposition::Rejected(format!(
                    "Invalid Lamport rank: expected {}, got {}", expected_lamp, mutation.body.lamport
                ));
            }

            if mutation.body.epoch != expected_epoch {
                return IngressDisposition::Rejected(format!(
                    "Invalid Epoch: expected {}, got {}", expected_epoch, mutation.body.epoch
                ));
            }
        }

        // --- 5. ADMIT AND APPLY TO STATE ---
        let admitted_id = mutation.id;
        self.apply_mutation_internal(mutation);

        // --- 6. RECURSIVELY RELEASE DEPENDENCY BUFFER ---
        self.release_unblocked_dependencies(admitted_id);

        IngressDisposition::AdmittedApplied(admitted_id)
    }

    pub fn ingest_mutation_with_admissions(&mut self, mutation: Mutation) -> (IngressDisposition, Vec<MutationID>) {
        // --- 1. PREFLIGHT VERIFICATION ---
        let expected_id = hash_mutation_body(&mutation.body);
        if mutation.id != expected_id {
            return (IngressDisposition::Invalid("Forged MutationID: ID != hash(Body)".into()), Vec::new());
        }

        // --- 2. LOCAL KNOWLEDGE LOOKUP (DUPLICATE CHECK) ---
        if self.dag.contains_key(&mutation.id) {
            return (IngressDisposition::Duplicate(mutation.id), Vec::new());
        }

        // --- 3. DEPENDENCY CHECK (MISSING PARENTS) ---
        let mut missing_parents = Vec::new();
        for p in &mutation.body.parents {
            if !self.dag.contains_key(p) {
                missing_parents.push(*p);
            }
        }

        if !missing_parents.is_empty() {
            let missing_set: HashSet<MutationID> = missing_parents.iter().copied().collect();
            self.dependency_buffer.insert(mutation.id, (mutation, missing_set));
            return (IngressDisposition::DependencyGap { missing_parents }, Vec::new());
        }

        // --- 4. CAUSAL ADMISSIBILITY VALIDATION ---
        for i in 1..mutation.body.parents.len() {
            if mutation.body.parents[i - 1] >= mutation.body.parents[i] {
                return (IngressDisposition::Rejected("Parents not strictly sorted without duplicates".into()), Vec::new());
            }
        }

        if mutation.body.parents.is_empty() {
            if mutation.body.lamport != 0 || mutation.body.epoch != 0 {
                return (IngressDisposition::Rejected("Genesis mutation must have Lamport 0 and Epoch 0".into()), Vec::new());
            }
        } else {
            if mutation.body.lamport == 0 {
                return (IngressDisposition::Rejected("Non-genesis mutation cannot have Lamport 0".into()), Vec::new());
            }

            let mut max_p_lamp = 0u64;
            let mut max_p_epoch = 0u64;

            for p_id in &mutation.body.parents {
                let parent = self.dag.get(p_id).expect("Parent must be present in DAG");
                if parent.body.lamport > max_p_lamp {
                    max_p_lamp = parent.body.lamport;
                }
                if parent.body.epoch > max_p_epoch {
                    max_p_epoch = parent.body.epoch;
                }
            }

            let expected_lamp = max_p_lamp + 1;
            let expected_epoch = if mutation.body.is_resurrect { max_p_epoch + 1 } else { max_p_epoch };

            if mutation.body.lamport != expected_lamp {
                return (IngressDisposition::Rejected(format!(
                    "Invalid Lamport rank: expected {}, got {}", expected_lamp, mutation.body.lamport
                )), Vec::new());
            }

            if mutation.body.epoch != expected_epoch {
                return (IngressDisposition::Rejected(format!(
                    "Invalid Epoch: expected {}, got {}", expected_epoch, mutation.body.epoch
                )), Vec::new());
            }
        }

        // --- 5. ADMIT AND APPLY TO STATE ---
        let admitted_id = mutation.id;
        self.apply_mutation_internal(mutation);
        let mut all_admitted = vec![admitted_id];

        // --- 6. RECURSIVELY RELEASE DEPENDENCY BUFFER ---
        let unblocked = self.release_unblocked_dependencies(admitted_id);
        all_admitted.extend(unblocked);

        (IngressDisposition::AdmittedApplied(admitted_id), all_admitted)
    }

    fn apply_mutation_internal(&mut self, mutation: Mutation) {
        let m_id = mutation.id;

        // Apply CRDT state modification
        if mutation.body.is_resurrect {
            self.crdt_state.clear();
        }

        self.current_lamport = self.current_lamport.max(mutation.body.lamport);

        let (obj_id, opt_val) = match &mutation.body.payload {
            CrdtPayload::AddLWW { id, value } => (*id, Some(value.clone())),
            CrdtPayload::RemoveLWW { id } => (*id, None),
            CrdtPayload::Tombstone { id } => (*id, None),
        };

        let apply = match self.crdt_state.get(&obj_id) {
            Some((_, old_e, old_l, old_id)) => {
                mutation.body.epoch > *old_e ||
                (mutation.body.epoch == *old_e && mutation.body.lamport > *old_l) ||
                (mutation.body.epoch == *old_e && mutation.body.lamport == *old_l && mutation.id > *old_id)
            }
            None => true,
        };

        if apply {
            self.crdt_state.insert(obj_id, (opt_val, mutation.body.epoch, mutation.body.lamport, mutation.id));
        }

        // Update frontier: Remove parents, insert new mutation
        for p in &mutation.body.parents {
            self.frontier.remove(p);
        }
        self.frontier.insert(m_id);

        // Insert into DAG
        self.dag.insert(m_id, mutation);
    }

    fn release_unblocked_dependencies(&mut self, newly_admitted: MutationID) -> Vec<MutationID> {
        let mut newly_admitted_queue = std::collections::VecDeque::new();
        newly_admitted_queue.push_back(newly_admitted);
        let mut unblocked = Vec::new();

        while let Some(admitted_id) = newly_admitted_queue.pop_front() {
            let mut ready_ids = Vec::new();
            for (buf_id, (_, missing_set)) in self.dependency_buffer.iter_mut() {
                missing_set.remove(&admitted_id);
                if missing_set.is_empty() {
                    ready_ids.push(*buf_id);
                }
            }

            for ready_id in ready_ids {
                if let Some((ready_mutation, _)) = self.dependency_buffer.remove(&ready_id) {
                    let next_admitted = ready_mutation.id;
                    self.apply_mutation_internal(ready_mutation);
                    unblocked.push(next_admitted);
                    newly_admitted_queue.push_back(next_admitted);
                }
            }
        }
        unblocked
    }

    /// Computes the exact constitutional Checkpoint for the current node state
    pub fn compute_current_checkpoint(&mut self) -> Checkpoint {
        let mut state_bytes = Vec::new();
        for (k, (v, e, l, id)) in &self.crdt_state {
            state_bytes.extend_from_slice(k);
            if let Some(val) = v {
                state_bytes.push(1);
                state_bytes.extend_from_slice(val);
            } else {
                state_bytes.push(0);
            }
            state_bytes.extend_from_slice(&e.to_le_bytes());
            state_bytes.extend_from_slice(&l.to_le_bytes());
            state_bytes.extend_from_slice(id);
        }
        let state_root = hash_canonical(DOMAIN_STATE_ROOT, &state_bytes);

        let mut causal_bytes = Vec::new();
        for id in self.dag.keys() {
            causal_bytes.extend_from_slice(id);
        }
        let causal_root = hash_canonical(DOMAIN_CAUSAL_ROOT, &causal_bytes);

        let mut admission_bytes = Vec::new();
        for id in self.dag.keys() {
            admission_bytes.extend_from_slice(id);
            admission_bytes.push(1u8); // Admitted
        }
        let admission_root = hash_canonical(DOMAIN_ADMISSION_ROOT, &admission_bytes);

        let mut max_lamport = 0u64;
        let mut max_epoch = 0u64;
        for m in self.dag.values() {
            if m.body.lamport > max_lamport { max_lamport = m.body.lamport; }
            if m.body.epoch > max_epoch { max_epoch = m.body.epoch; }
        }

        let body = CheckpointBody {
            state_root,
            causal_root,
            admission_root,
            frontier: self.frontier.iter().copied().collect(),
            boundary: Boundary { max_epoch, max_lamport },
        };
        let id = hash_checkpoint_body(&body);
        let cp = Checkpoint { id, body };
        self.latest_checkpoint = Some(cp.clone());
        cp
    }
}
