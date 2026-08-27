use std::collections::BTreeMap;
use crate::model::MutationID;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};

pub type StateCommitment = [u8; 32];
pub type AccumulatorRoot = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmtInclusionProof {
    /// 256 sibling hashes from leaf level (index 255) up to root child (index 0).
    pub siblings: Vec<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmtNonInclusionProof {
    /// 256 sibling hashes demonstrating that the key slot is currently empty (E_256).
    pub siblings: Vec<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtUpdateResult {
    /// Case 1: Slot was empty, new leaf inserted -> root advanced.
    Inserted(AccumulatorRoot),
    /// Case 2: Slot already contains identical StateCommitment -> idempotent no-op.
    NoOp(AccumulatorRoot),
    /// Case 3: Slot contains a different StateCommitment -> collision/forgery rejected.
    Conflict,
}

#[derive(Debug, Clone, Default)]
pub struct SparseMerkleTree {
    pub entries: BTreeMap<[u8; 32], StateCommitment>,
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new() }
    }

    pub fn insert_or_verify(
        &mut self,
        mutation_id: &MutationID,
        commitment: &StateCommitment,
    ) -> Result<SmtUpdateResult, &'static str> {
        let key = sha256_smt_key(mutation_id);
        if let Some(existing) = self.entries.get(&key) {
            if existing == commitment {
                return Ok(SmtUpdateResult::NoOp(self.root()));
            } else {
                return Ok(SmtUpdateResult::Conflict);
            }
        }
        self.entries.insert(key, *commitment);
        Ok(SmtUpdateResult::Inserted(self.root()))
    }

    pub fn root(&self) -> AccumulatorRoot {
        if self.entries.is_empty() {
            return [0u8; 32];
        }
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/SMT_TREE_ROOT/v1");
        for (k, v) in &self.entries {
            hasher.update(k);
            hasher.update(v);
        }
        hasher.finalize().into()
    }
}

#[inline]
pub fn sha256_smt_key(mutation_id: &MutationID) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"NEX/SMT_KEY/v1");
    hasher.update(mutation_id);
    hasher.finalize().into()
}

#[inline]
pub fn sha256_smt_leaf(state_commitment: &StateCommitment) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"NEX/SMT_LEAF/v1");
    hasher.update(state_commitment);
    hasher.finalize().into()
}

#[inline]
pub fn sha256_smt_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"NEX/SMT_NODE/v1");
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Verifies that a (mutation_id, state_commitment) exists under `root`.
pub fn verify_smt_inclusion(
    root: &[u8; 32],
    mutation_id: &MutationID,
    state_commitment: &StateCommitment,
    proof: &SmtInclusionProof,
) -> bool {
    if proof.siblings.len() != 256 {
        return false;
    }
    let key = sha256_smt_key(mutation_id);
    let mut current_hash = sha256_smt_leaf(state_commitment);

    for (bit_idx, sibling) in proof.siblings.iter().enumerate().rev() {
        let bit = (key[bit_idx / 8] >> (7 - (bit_idx % 8))) & 1;
        current_hash = if bit == 0 {
            sha256_smt_node(&current_hash, sibling)
        } else {
            sha256_smt_node(sibling, &current_hash)
        };
    }

    &current_hash == root
}

/// Evaluates the 3-state SMT insertion algebra:
/// Case 1 (Empty): Proof proves slot was [0; 32] -> Computes and returns Inserted(new_root).
/// Case 2 (Same): Proof proves slot already has state_commitment -> Returns NoOp(current_root).
/// Case 3 (Conflict): Slot occupied with different commitment -> Returns Conflict.
pub fn insert_or_verify_smt(
    current_root: &[u8; 32],
    mutation_id: &MutationID,
    state_commitment: &StateCommitment,
    proof_siblings: &[[u8; 32]],
) -> Result<SmtUpdateResult, &'static str> {
    if proof_siblings.len() != 256 {
        return Err("Invalid SMT proof depth. Must be 256.");
    }
    let key = sha256_smt_key(mutation_id);

    // 1. Check if the slot is currently empty (E_256 = [0u8; 32])
    let mut empty_hash = [0u8; 32];
    for (bit_idx, sibling) in proof_siblings.iter().enumerate().rev() {
        let bit = (key[bit_idx / 8] >> (7 - (bit_idx % 8))) & 1;
        empty_hash = if bit == 0 {
            sha256_smt_node(&empty_hash, sibling)
        } else {
            sha256_smt_node(sibling, &empty_hash)
        };
    }

    if &empty_hash == current_root {
        // CASE 1: Empty slot -> Advance root with new leaf value
        let mut new_hash = sha256_smt_leaf(state_commitment);
        for (bit_idx, sibling) in proof_siblings.iter().enumerate().rev() {
            let bit = (key[bit_idx / 8] >> (7 - (bit_idx % 8))) & 1;
            new_hash = if bit == 0 {
                sha256_smt_node(&new_hash, sibling)
            } else {
                sha256_smt_node(sibling, &new_hash)
            };
        }
        return Ok(SmtUpdateResult::Inserted(new_hash));
    }

    // 2. Check if the slot already contains the identical StateCommitment
    let mut expected_leaf_hash = sha256_smt_leaf(state_commitment);
    for (bit_idx, sibling) in proof_siblings.iter().enumerate().rev() {
        let bit = (key[bit_idx / 8] >> (7 - (bit_idx % 8))) & 1;
        expected_leaf_hash = if bit == 0 {
            sha256_smt_node(&expected_leaf_hash, sibling)
        } else {
            sha256_smt_node(sibling, &expected_leaf_hash)
        };
    }

    if &expected_leaf_hash == current_root {
        // CASE 2: Identical commitment already in place -> Idempotent No-Op
        return Ok(SmtUpdateResult::NoOp(*current_root));
    }

    // CASE 3: Slot occupied with a different commitment or invalid path
    Ok(SmtUpdateResult::Conflict)
}
