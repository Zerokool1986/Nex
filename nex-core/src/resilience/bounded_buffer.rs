use std::collections::{BTreeMap, HashSet};
use crate::model::{Mutation, MutationID};

pub const MAX_DEPENDENCY_BUFFER_ENTRIES: usize = 512;
pub const DEPENDENCY_TTL_EPOCHS: u64 = 300;

#[derive(Debug, Clone, Default)]
pub struct BoundedDependencyBuffer {
    /// In-flight orphan mutations: mutation_id -> (mutation, missing_parents, inserted_epoch)
    pub entries: BTreeMap<MutationID, (Mutation, HashSet<MutationID>, u64)>,
}

impl BoundedDependencyBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Inserts a mutation into the dependency buffer. If at maximum capacity, the oldest entry is evicted.
    pub fn insert(&mut self, mutation: Mutation, missing: HashSet<MutationID>, current_epoch: u64) -> Option<MutationID> {
        let mut evicted = None;

        if self.entries.len() >= MAX_DEPENDENCY_BUFFER_ENTRIES && !self.entries.contains_key(&mutation.id) {
            // Find and evict oldest entry
            let oldest_id = self.entries
                .iter()
                .min_by_key(|(_, (_, _, epoch))| *epoch)
                .map(|(id, _)| *id);

            if let Some(oldest) = oldest_id {
                self.entries.remove(&oldest);
                evicted = Some(oldest);
            }
        }

        self.entries.insert(mutation.id, (mutation, missing, current_epoch));
        evicted
    }

    /// Prunes entries that have exceeded TTL
    pub fn prune_expired(&mut self, current_epoch: u64) -> usize {
        let mut expired_ids = Vec::new();
        for (id, (_, _, inserted_epoch)) in &self.entries {
            if current_epoch.saturating_sub(*inserted_epoch) > DEPENDENCY_TTL_EPOCHS {
                expired_ids.push(*id);
            }
        }

        let count = expired_ids.len();
        for id in expired_ids {
            self.entries.remove(&id);
        }
        count
    }

    /// Releases any mutations that are now unblocked by the newly admitted parent
    pub fn release_unblocked(&mut self, newly_admitted: &MutationID) -> Vec<Mutation> {
        let mut ready_ids = Vec::new();
        for (id, (_, missing, _)) in self.entries.iter_mut() {
            missing.remove(newly_admitted);
            if missing.is_empty() {
                ready_ids.push(*id);
            }
        }

        let mut ready_mutations = Vec::new();
        for id in ready_ids {
            if let Some((mutation, _, _)) = self.entries.remove(&id) {
                ready_mutations.push(mutation);
            }
        }
        ready_mutations
    }
}
