use std::collections::{BTreeMap, BTreeSet};
use sha2::{Sha256, Digest};

pub const DEFAULT_TOMBSTONE_GRACE_EPOCHS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkReference {
    pub chunk_hash: [u8; 32],
    pub referenced_at_epoch: u64,
    pub tombstoned_at_epoch: Option<u64>,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CasCompactor {
    pub chunks: BTreeMap<[u8; 32], ChunkReference>,
    pub tombstone_grace_epochs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionReport {
    pub total_chunks_before: usize,
    pub total_chunks_after: usize,
    pub chunks_reclaimed: usize,
    pub bytes_reclaimed: usize,
    pub remaining_active_bytes: usize,
}

impl CasCompactor {
    pub fn new(grace_epochs: u64) -> Self {
        Self {
            chunks: BTreeMap::new(),
            tombstone_grace_epochs: grace_epochs,
        }
    }

    pub fn insert_chunk(&mut self, chunk_hash: [u8; 32], epoch: u64, size: usize) {
        self.chunks.entry(chunk_hash).or_insert(ChunkReference {
            chunk_hash,
            referenced_at_epoch: epoch,
            tombstoned_at_epoch: None,
            size_bytes: size,
        });
    }

    pub fn mark_tombstone(&mut self, chunk_hash: &[u8; 32], tombstone_epoch: u64) {
        if let Some(entry) = self.chunks.get_mut(chunk_hash) {
            if entry.tombstoned_at_epoch.is_none() {
                entry.tombstoned_at_epoch = Some(tombstone_epoch);
            }
        }
    }

    /// Executes generational garbage collection at `current_epoch`.
    /// Reclaims chunks whose tombstone age exceeds `tombstone_grace_epochs`.
    pub fn collect_garbage(&mut self, current_epoch: u64, active_references: &BTreeSet<[u8; 32]>) -> CompactionReport {
        let total_before = self.chunks.len();
        let mut chunks_to_remove = Vec::new();
        let mut bytes_reclaimed = 0;
        let mut remaining_bytes = 0;

        for (&hash, ref_entry) in self.chunks.iter() {
            let is_actively_referenced = active_references.contains(&hash);

            if !is_actively_referenced {
                if let Some(t_epoch) = ref_entry.tombstoned_at_epoch {
                    if current_epoch >= t_epoch + self.tombstone_grace_epochs {
                        chunks_to_remove.push(hash);
                        bytes_reclaimed += ref_entry.size_bytes;
                        continue;
                    }
                }
            }

            remaining_bytes += ref_entry.size_bytes;
        }

        for hash in &chunks_to_remove {
            self.chunks.remove(hash);
        }

        CompactionReport {
            total_chunks_before: total_before,
            total_chunks_after: self.chunks.len(),
            chunks_reclaimed: chunks_to_remove.len(),
            bytes_reclaimed,
            remaining_active_bytes: remaining_bytes,
        }
    }
}
