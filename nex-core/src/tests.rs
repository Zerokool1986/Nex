use crate::model::*;
use crate::serialize::{CanonicalSerialize, SerializationError};
use crate::hash::*;
use crate::accumulator::*;

#[test]
fn test_vector_01_empty_parents() {
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let mut buf = Vec::new();
    assert!(mb.canonical_serialize(&mut buf).is_ok());
    assert_eq!(buf.len(), 1 + 32 + 4 + 8 + 8 + 1 + (1 + 32)); // 87 bytes
}

#[test]
fn test_vector_02_one_parent() {
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![[1u8; 32]],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let mut buf = Vec::new();
    assert!(mb.canonical_serialize(&mut buf).is_ok());
    assert_eq!(buf.len(), 87 + 32); // 119 bytes
}

#[test]
fn test_vector_03_two_parents_canonical_order() {
    let p1 = [1u8; 32];
    let p2 = [2u8; 32];
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![p1, p2],
        lamport: 2,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let mut buf = Vec::new();
    assert!(mb.canonical_serialize(&mut buf).is_ok());
}

#[test]
fn test_vector_04_two_parents_reversed_fails() {
    let p1 = [1u8; 32];
    let p2 = [2u8; 32];
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![p2, p1], // Out of order!
        lamport: 2,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let mut buf = Vec::new();
    assert_eq!(mb.canonical_serialize(&mut buf), Err(SerializationError::UnsortedParents));
}

#[test]
fn test_vector_05_duplicate_parents_rejected() {
    let p1 = [1u8; 32];
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![p1, p1], // Duplicate!
        lamport: 2,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let mut buf = Vec::new();
    assert_eq!(mb.canonical_serialize(&mut buf), Err(SerializationError::DuplicateParents));
}

#[test]
fn test_vector_06_lamport_zero() {
    let mut buf = Vec::new();
    (0u64).canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf, vec![0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_vector_07_lamport_u64_max() {
    let mut buf = Vec::new();
    u64::MAX.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf, vec![0xFF; 8]);
}

#[test]
fn test_vector_08_epoch_u64_max() {
    let mut buf = Vec::new();
    u64::MAX.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf, vec![0xFF; 8]);
}

#[test]
fn test_vector_09_boolean_false() {
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let mut buf = Vec::new();
    mb.canonical_serialize(&mut buf).unwrap();
    // Offset for boolean: 1(version) + 32(author) + 4(parents_len) + 8(lamport) + 8(epoch) = byte index 53
    assert_eq!(buf[53], 0x00);
}

#[test]
fn test_vector_10_boolean_true() {
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: true,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let mut buf = Vec::new();
    mb.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf[53], 0x01);
}

#[test]
fn test_vector_11_empty_payload() {
    let payload = CrdtPayload::AddLWW { id: [0u8; 32], value: vec![] };
    let mut buf = Vec::new();
    payload.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf.len(), 1 + 32 + 4);
    assert_eq!(&buf[33..37], &[0, 0, 0, 0]); // 0-length
}

#[test]
fn test_vector_12_one_byte_payload() {
    let payload = CrdtPayload::AddLWW { id: [0u8; 32], value: vec![0x42] };
    let mut buf = Vec::new();
    payload.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf.len(), 1 + 32 + 4 + 1);
    assert_eq!(buf[37], 0x42);
}

#[test]
fn test_vector_13_large_payload() {
    let payload = CrdtPayload::AddLWW { id: [0u8; 32], value: vec![0xAA; 1000] };
    let mut buf = Vec::new();
    payload.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf.len(), 1 + 32 + 4 + 1000);
}

#[test]
fn test_vector_14_add_lww_discriminant() {
    let payload = CrdtPayload::AddLWW { id: [0u8; 32], value: vec![] };
    let mut buf = Vec::new();
    payload.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf[0], 0x01);
}

#[test]
fn test_vector_15_remove_lww_discriminant() {
    let payload = CrdtPayload::RemoveLWW { id: [0u8; 32] };
    let mut buf = Vec::new();
    payload.canonical_serialize(&mut buf).unwrap();
    assert_eq!(buf[0], 0x02);
}

#[test]
fn test_vector_16_known_mutation_body_to_mutation_id() {
    let mb = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let id = hash_mutation_body(&mb);
    assert_eq!(
        hex::encode(id),
        "d7632356a5233312d254c47eed29db541190c1a77d78c6621c8788f2b9b86a51"
    );
}

#[test]
fn test_vector_17_known_state_encoding_to_state_commitment() {
    let se = StateEncoding {
        mutation_id: [1u8; 32],
        lamport: 10,
        epoch: 2,
        is_resurrect: false,
        payload: CrdtPayload::RemoveLWW { id: [0u8; 32] },
    };
    let c = hash_state_encoding(&se);
    assert_eq!(
        hex::encode(c),
        "65d09827c4bec0b484917229f3dbe76f49a9fb6acb21aa32f33d308e8e440cef"
    );
}

#[test]
fn test_vector_18_known_smt_key() {
    let mid = [1u8; 32];
    let k = sha256_smt_key(&mid);
    assert_eq!(
        hex::encode(k),
        "15535a3854f21d372e338491b816d3ddba4664c1dd1d30b8576552c7ccc609a9"
    );
}

#[test]
fn test_vector_19_known_smt_leaf() {
    let sc = [42u8; 32];
    let leaf = sha256_smt_leaf(&sc);
    assert_eq!(
        hex::encode(leaf),
        "84092011c9b86af64958344a2d82bb8890261ff234474b2143cdbc08d78e18c1"
    );
}

#[test]
fn test_vector_20_deterministic_smt_root_set() {
    // Computes SMT root of { (m1, c1) }
    let mut empty_hashes = vec![[0u8; 32]; 257];
    for d in (0..256).rev() {
        empty_hashes[d] = sha256_smt_node(&empty_hashes[d + 1], &empty_hashes[d + 1]);
    }
    let empty_root = empty_hashes[0];

    let mut siblings = vec![[0u8; 32]; 256];
    for d in 0..256 {
        siblings[d] = empty_hashes[d + 1];
    }

    let m1 = [1u8; 32];
    let c1 = [42u8; 32];

    let res = insert_or_verify_smt(&empty_root, &m1, &c1, &siblings).unwrap();
    let root = match res {
        SmtUpdateResult::Inserted(r) => r,
        _ => panic!("Expected Inserted"),
    };

    assert_eq!(
        hex::encode(root),
        "b665a874ac0aa9ffc21242571c9014da40b69f71cfec77d8608b3f5a1c47c3e2"
    );
}

#[test]
fn test_preimage_invariance_after_id_population() {
    let body = MutationBody {
        author: [7u8; 32],
        parents: vec![[1u8; 32]],
        lamport: 10,
        epoch: 2,
        is_resurrect: true,
        payload: CrdtPayload::AddLWW { id: [3u8; 32], value: vec![1, 2, 3] },
    };

    let id = hash_mutation_body(&body);
    let m = Mutation::new(id, body.clone());

    assert_eq!(
        id,
        hash_mutation(&m),
        "Preimage must be invariant and identical to hash_mutation_body!"
    );
}
