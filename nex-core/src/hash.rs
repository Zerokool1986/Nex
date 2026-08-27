use sha2::{Sha256, Digest};
use crate::serialize::CanonicalSerialize;

pub const DOMAIN_MUTATION: &[u8] = b"NEX/MUTATION/v1";
pub const DOMAIN_STATE_COMMITMENT: &[u8] = b"NEX/STATE_COMMITMENT/v1";
pub const DOMAIN_CHECKPOINT: &[u8] = b"NEX/CHECKPOINT/v1";
pub const DOMAIN_STATE_ROOT: &[u8] = b"NEX/STATE_ROOT/v1";
pub const DOMAIN_CAUSAL_ROOT: &[u8] = b"NEX/CAUSAL_ROOT/v1";
pub const DOMAIN_ADMISSION_ROOT: &[u8] = b"NEX/ADMISSION_ROOT/v1";
pub const DOMAIN_INPUT_COMMITMENT: &[u8] = b"NEX/INPUT_COMMITMENT/v1";
pub const DOMAIN_FRONTIER_COMMITMENT: &[u8] = b"NEX/FRONTIER_COMMITMENT/v1";
pub const DOMAIN_ZKVM_JOURNAL: &[u8] = b"NEX/ZKVM_JOURNAL/v1";

pub fn hash_canonical<T: CanonicalSerialize>(domain: &[u8], item: &T) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    let mut buf = Vec::new();
    item.canonical_serialize(&mut buf).expect("Canonical serialization failed");
    hasher.update(&buf);
    hasher.finalize().into()
}

pub fn hash_mutation_body(body: &crate::model::MutationBody) -> [u8; 32] {
    hash_canonical(DOMAIN_MUTATION, body)
}

pub fn hash_mutation(mutation: &crate::model::Mutation) -> [u8; 32] {
    hash_canonical(DOMAIN_MUTATION, &mutation.body)
}

pub fn hash_state_encoding(state: &crate::model::StateEncoding) -> [u8; 32] {
    hash_canonical(DOMAIN_STATE_COMMITMENT, state)
}

pub fn hash_checkpoint_body(body: &crate::model::CheckpointBody) -> [u8; 32] {
    hash_canonical(DOMAIN_CHECKPOINT, body)
}

pub fn hash_checkpoint(checkpoint: &crate::model::Checkpoint) -> [u8; 32] {
    hash_canonical(DOMAIN_CHECKPOINT, &checkpoint.body)
}
