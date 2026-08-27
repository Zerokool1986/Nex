pub mod model;
pub mod serialize;
pub mod hash;
pub mod accumulator;
pub mod sync;
pub mod identity;
pub mod discovery;
pub mod transport;
pub mod resilience;
pub mod apps;
pub mod runtime;
pub mod storage;
pub mod object;
pub mod api;
pub mod ipc;
pub mod cli;
pub mod ffi;
pub mod product;
pub mod crdt;
pub mod cbor_strict;

use std::cmp::Ordering;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

/// HashRef as defined in NEX-PROTOCOL-WIRE-SPEC-v1.0
/// Structurally [AlgorithmID: uint, Digest: bstr]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashRef {
    pub algorithm_id: u64,
    pub digest: Vec<u8>,
}

impl PartialOrd for HashRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HashRef {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.algorithm_id.cmp(&other.algorithm_id) {
            Ordering::Equal => self.digest.cmp(&other.digest),
            other => other,
        }
    }
}

impl HashRef {
    pub fn new_sha256(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        HashRef {
            algorithm_id: 1,
            digest: hasher.finalize().to_vec(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.algorithm_id == 1 && self.digest.len() != 32 {
            return Err("INVALID_HASH_REF");
        }
        if self.algorithm_id != 1 {
            return Err("INVALID_HASH_REF");
        }
        Ok(())
    }
}

pub fn resolve_genesis_collision(candidates: &[HashRef]) -> Option<HashRef> {
    candidates.iter().min().cloned()
}

pub fn validate_identity_genesis_authority(
    author_device_key: &[u8],
    root_device_key: &[u8],
    is_identity_context_nil: bool,
    is_capability_ref_nil: bool,
) -> Result<(), &'static str> {
    if author_device_key == root_device_key && is_identity_context_nil && is_capability_ref_nil {
        Ok(())
    } else {
        Err("INVALID_AUTHORITY")
    }
}

