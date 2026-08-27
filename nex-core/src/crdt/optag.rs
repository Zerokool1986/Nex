use crate::HashRef;
use sha2::{Sha256, Digest};
use serde::Serialize;
use super::types::{OperationIndex, OperationBody};

#[derive(Serialize)]
struct OpTagPayload<'a> {
    mutation_id: &'a HashRef,
    operation_index: &'a OperationIndex,
    body: &'a OperationBody,
}

pub fn compute_optag(
    mutation_id: &HashRef,
    operation_index: &OperationIndex,
    body: &OperationBody,
) -> HashRef {
    let payload = (mutation_id, operation_index, body);
    let mut cbor_bytes = Vec::new();
    ciborium::into_writer(&payload, &mut cbor_bytes).expect("Failed to serialize to CBOR");

    let mut hasher = Sha256::new();
    hasher.update(b"NEX/OPTAG/v1");
    hasher.update(&cbor_bytes);
    
    HashRef {
        algorithm_id: 1,
        digest: hasher.finalize().to_vec(),
    }
}
