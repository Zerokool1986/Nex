pub mod cbor_strict;
use std::cmp::Ordering;
use sha2::{Sha256, Digest};

/// HashRef as defined in NEX-PROTOCOL-WIRE-SPEC-v1.0
/// Structurally [AlgorithmID: uint, Digest: bstr]
#[derive(Debug, Clone, PartialEq, Eq)]
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
        // Bytewise lexicographic order of the CBOR encoded bytes
        // In this minimal primitive, we sort by alg ID then bytes.
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
            return Err("INVALID_HASH_REF"); // v1.0 only permits alg 1
        }
        Ok(())
    }
}

/// Identifiers derivation
pub fn derive_object_id(genesis_descriptor_cbor: &[u8]) -> HashRef {
    let mut prefix = b"NEX/OBJECT_ID/v1".to_vec();
    prefix.extend_from_slice(genesis_descriptor_cbor);
    HashRef::new_sha256(&prefix)
}

pub fn derive_mutation_id(mutation_body_cbor: &[u8]) -> HashRef {
    let mut prefix = b"NEX/MUTATION_ID/v1".to_vec();
    prefix.extend_from_slice(mutation_body_cbor);
    HashRef::new_sha256(&prefix)
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
