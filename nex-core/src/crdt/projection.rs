use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use super::types::RegisterState;
use crate::HashRef;
use serde::Serialize;

pub struct CrdtStateMap {
    pub adds_map: BTreeMap<Vec<u8>, (HashRef, Vec<u8>)>,
    pub tombstones_arr: Vec<HashRef>,
}

pub fn project(register_state: &BTreeMap<Vec<u8>, RegisterState>) -> CrdtStateMap {
    let mut adds_map = BTreeMap::new();
    let mut tombstones_arr = Vec::new();
    
    for (key, state) in register_state {
        match state {
            RegisterState::Add { op_tag, payload, .. } => {
                adds_map.insert(key.clone(), (op_tag.clone(), payload.clone()));
            }
            RegisterState::Remove { op_tag, .. } => {
                tombstones_arr.push(op_tag.clone());
            }
        }
    }
    
    tombstones_arr.sort();
    tombstones_arr.dedup(); // Ensure uniqueness if necessary, though operations might be unique
    
    CrdtStateMap {
        adds_map,
        tombstones_arr,
    }
}

pub fn compute_state_commitment(state_map: &CrdtStateMap) -> HashRef {
    let mut cbor_bytes = Vec::new();
    ciborium::into_writer(state_map, &mut cbor_bytes).expect("Failed to serialize StateEncoding");

    let mut hasher = Sha256::new();
    hasher.update(b"NEX/STATE_COMMITMENT/v1");
    hasher.update(&cbor_bytes);
    
    HashRef {
        algorithm_id: 1,
        digest: hasher.finalize().to_vec(),
    }
}
