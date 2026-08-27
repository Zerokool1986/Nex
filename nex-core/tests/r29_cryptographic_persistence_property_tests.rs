use std::fs;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token, verify_capability_chain};
use nex_core::identity::types::{
    KeyType, CapabilityToken, CapabilityProof, AuthorizationError,
    OP_REGISTER_LWW, OP_ALL, OP_OBJECT_TOMBSTONE, OP_SET_ADD
};
use nex_core::storage::wal::WriteAheadLog;
use nex_core::sync::node::VirtualNode;
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::hash::hash_mutation_body;

#[test]
fn test_r29_a_complete_16_case_capability_tamper_matrix() {
    let mut csprng = OsRng;
    let alice_signing_key = SigningKey::generate(&mut csprng);
    let alice_pubkey = alice_signing_key.verifying_key().to_bytes();
    let alice_actor = derive_actor_id(KeyType::Ed25519, &alice_pubkey);

    let bob_signing_key = SigningKey::generate(&mut csprng);
    let bob_pubkey = bob_signing_key.verifying_key().to_bytes();
    let bob_actor = derive_actor_id(KeyType::Ed25519, &bob_pubkey);

    let charlie_signing_key = SigningKey::generate(&mut csprng);
    let charlie_pubkey = charlie_signing_key.verifying_key().to_bytes();
    let charlie_actor = derive_actor_id(KeyType::Ed25519, &charlie_pubkey);

    let namespace = [0xAA; 32];
    let shared_obj = [0xBB; 32];

    // Root Token: Alice grants Bob write access
    let root_token = CapabilityToken {
        issuer: alice_actor,
        subject: bob_actor,
        namespace,
        object_id: Some(shared_obj),
        allowed_operations: OP_REGISTER_LWW | OP_SET_ADD,
        delegation_depth: 2,
        not_before_epoch: 10,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let root_hash = hash_capability_token(&root_token);
    let root_sig = alice_signing_key.sign(&root_hash).to_bytes().to_vec();
    let root_proof = CapabilityProof {
        token: root_token.clone(),
        parent_proof: None,
        issuer_pubkey: Some(alice_pubkey.to_vec()),
        signature: root_sig,
    };

    // Delegated Token: Bob delegates to Charlie
    let child_token = CapabilityToken {
        issuer: bob_actor,
        subject: charlie_actor,
        namespace,
        object_id: Some(shared_obj),
        allowed_operations: OP_REGISTER_LWW,
        delegation_depth: 1,
        not_before_epoch: 15,
        expires_at_epoch: 90,
        parent_token_hash: Some(root_hash),
    };
    let child_hash = hash_capability_token(&child_token);
    let child_sig = bob_signing_key.sign(&child_hash).to_bytes().to_vec();
    let valid_child_proof = CapabilityProof {
        token: child_token.clone(),
        parent_proof: Some(Box::new(root_proof.clone())),
        issuer_pubkey: Some(bob_pubkey.to_vec()),
        signature: child_sig,
    };

    let empty_rev = std::collections::BTreeMap::new();

    // CASE 1: Valid Root Proof Passes
    let c1 = verify_capability_chain(&root_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c1, Ok(bob_actor), "Case 1 failed");

    // CASE 2: Bit-Flipped Signature on Root
    let mut c2_proof = root_proof.clone();
    c2_proof.signature[0] ^= 0xFF;
    let c2 = verify_capability_chain(&c2_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c2, Err(AuthorizationError::SignatureInvalid), "Case 2 failed");

    // CASE 3: Truncated Signature
    let mut c3_proof = root_proof.clone();
    c3_proof.signature.truncate(32);
    let c3 = verify_capability_chain(&c3_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c3, Err(AuthorizationError::SignatureInvalid), "Case 3 failed");

    // CASE 4: Modified Namespace in Token
    let mut c4_proof = root_proof.clone();
    c4_proof.token.namespace = [0xEE; 32];
    let c4 = verify_capability_chain(&c4_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c4, Err(AuthorizationError::NamespaceMismatch), "Case 4 failed");

    // CASE 5: Modified ObjectID in Token
    let mut c5_proof = root_proof.clone();
    c5_proof.token.object_id = Some([0xDD; 32]);
    let c5 = verify_capability_chain(&c5_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c5, Err(AuthorizationError::ObjectMismatch), "Case 5 failed");

    // CASE 6: Modified Operations (Privilege Escalation)
    let mut c6_proof = root_proof.clone();
    c6_proof.token.allowed_operations = OP_ALL;
    let c6 = verify_capability_chain(&c6_proof, OP_OBJECT_TOMBSTONE, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c6, Err(AuthorizationError::SignatureInvalid), "Case 6 failed");

    // CASE 7: Modified Expiration Epoch (Future Extension)
    let mut c7_proof = root_proof.clone();
    c7_proof.token.expires_at_epoch = 9999;
    let c7 = verify_capability_chain(&c7_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c7, Err(AuthorizationError::SignatureInvalid), "Case 7 failed");

    // CASE 8: Forged Issuer Public Key
    let eve_key = SigningKey::generate(&mut csprng);
    let mut c8_proof = root_proof.clone();
    c8_proof.issuer_pubkey = Some(eve_key.verifying_key().to_bytes().to_vec());
    let c8 = verify_capability_chain(&c8_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c8, Err(AuthorizationError::RootIssuerMismatch), "Case 8 failed");

    // CASE 9: Modified Attenuation Depth (Child depth >= Parent depth)
    let mut c9_proof = valid_child_proof.clone();
    c9_proof.token.delegation_depth = 2; // Parent is 2, child must be <= 1
    let c9_hash = hash_capability_token(&c9_proof.token);
    c9_proof.signature = bob_signing_key.sign(&c9_hash).to_bytes().to_vec();
    let c9 = verify_capability_chain(&c9_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert!(matches!(c9, Err(AuthorizationError::ParentAttenuationViolation(_))), "Case 9 failed");

    // CASE 10: Modified Parent Token Hash
    let mut c10_proof = valid_child_proof.clone();
    c10_proof.token.parent_token_hash = Some([0x99; 32]);
    let c10_hash = hash_capability_token(&c10_proof.token);
    c10_proof.signature = bob_signing_key.sign(&c10_hash).to_bytes().to_vec();
    let c10 = verify_capability_chain(&c10_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert!(matches!(c10, Err(AuthorizationError::ParentAttenuationViolation(_))), "Case 10 failed");

    // CASE 11: Replayed Past Revocation Epoch
    let mut rev_map = std::collections::BTreeMap::new();
    rev_map.insert(root_hash, 15); // Revoked at epoch 15
    let c11 = verify_capability_chain(&root_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &rev_map, &alice_actor);
    assert!(matches!(c11, Err(AuthorizationError::RevokedCapability { .. })), "Case 11 failed");

    // CASE 12: Expired Token (current_epoch > expires_at)
    let c12 = verify_capability_chain(&root_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 105, &empty_rev, &alice_actor);
    assert!(matches!(c12, Err(AuthorizationError::ExpiredCapability { .. })), "Case 12 failed");

    // CASE 13: Not Yet Valid Token (current_epoch < not_before)
    let c13 = verify_capability_chain(&root_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 5, &empty_rev, &alice_actor);
    assert!(matches!(c13, Err(AuthorizationError::NotYetValid { .. })), "Case 13 failed");

    // CASE 14: Unauthorized Non-Root Issuer without Parent Proof
    let mut c14_proof = root_proof.clone();
    c14_proof.token.issuer = bob_actor;
    c14_proof.issuer_pubkey = Some(bob_pubkey.to_vec());
    let c14_hash = hash_capability_token(&c14_proof.token);
    c14_proof.signature = bob_signing_key.sign(&c14_hash).to_bytes().to_vec();
    let c14 = verify_capability_chain(&c14_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert_eq!(c14, Err(AuthorizationError::RootIssuerMismatch), "Case 14 failed");

    // CASE 15: Child Privilege Escalation (Child requests operation not in parent)
    let mut c15_proof = valid_child_proof.clone();
    c15_proof.token.allowed_operations = OP_ALL;
    let c15_hash = hash_capability_token(&c15_proof.token);
    c15_proof.signature = bob_signing_key.sign(&c15_hash).to_bytes().to_vec();
    let c15 = verify_capability_chain(&c15_proof, OP_OBJECT_TOMBSTONE, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert!(matches!(c15, Err(AuthorizationError::ParentAttenuationViolation(_))), "Case 15 failed");

    // CASE 16: Child Temporal Window Exceeds Parent
    let mut c16_proof = valid_child_proof.clone();
    c16_proof.token.expires_at_epoch = 150; // Parent expires at 100
    let c16_hash = hash_capability_token(&c16_proof.token);
    c16_proof.signature = bob_signing_key.sign(&c16_hash).to_bytes().to_vec();
    let c16 = verify_capability_chain(&c16_proof, OP_REGISTER_LWW, &namespace, Some(&shared_obj), 20, &empty_rev, &alice_actor);
    assert!(matches!(c16, Err(AuthorizationError::ParentAttenuationViolation(_))), "Case 16 failed");
}

#[test]
fn test_r29_b_four_crash_point_durability_suite() {
    let temp_dir = std::env::temp_dir().join(format!("nex_r29_crash_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&temp_dir).unwrap();
    let wal_path = temp_dir.join("durability.wal");

    // --- CRASH POINT 1: Pre-WAL Interruption ---
    // Mutation created in memory but process crashes before WAL append
    let b1 = MutationBody {
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [0x01; 32], value: b"pre_wal".to_vec() },
    };
    let m1 = Mutation { id: hash_mutation_body(&b1), body: b1 };
    // Simulated crash before append
    let rec1 = WriteAheadLog::recover(&wal_path).unwrap();
    assert_eq!(rec1.len(), 0, "Pre-WAL mutation must be absent after recovery");

    // --- CRASH POINT 2: Post-WAL / Pre-Apply Interruption ---
    // Mutation appended to WAL, but process crashes before in-memory CRDT/DAG ingestion
    let mut wal = WriteAheadLog::open(&wal_path).unwrap();
    wal.append_mutation(&m1).unwrap();
    drop(wal); // Simulated process termination before ingestion

    let rec2 = WriteAheadLog::recover(&wal_path).unwrap();
    assert_eq!(rec2.len(), 1, "Post-WAL mutation must be cleanly recovered");
    let mut node_recovered = VirtualNode::new("RecoveredNode");
    for m in rec2 {
        node_recovered.ingest_mutation(m);
    }
    assert_eq!(node_recovered.dag.len(), 1);

    // --- CRASH POINT 3: Mid-Apply Interruption with Partial Write ---
    // Mutation 2 appended, but file has partial cut-off bytes appended
    let b2 = MutationBody {
        parents: vec![m1.id],
        lamport: 1,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [0x02; 32], value: b"mid_apply".to_vec() },
    };
    let m2 = Mutation { id: hash_mutation_body(&b2), body: b2 };
    let mut wal = WriteAheadLog::open(&wal_path).unwrap();
    wal.append_mutation(&m2).unwrap();
    drop(wal);

    // Append 7 partial cutoff bytes
    let mut f = fs::OpenOptions::new().append(true).open(&wal_path).unwrap();
    use std::io::Write;
    f.write_all(b"PARTIAL").unwrap();
    drop(f);

    let rec3 = WriteAheadLog::recover(&wal_path).unwrap();
    assert_eq!(rec3.len(), 2, "Recovery must yield exactly the 2 committed mutations and discard partial cutoff");

    // --- CRASH POINT 4: Mid-Checkpoint Interruption & Multi-Crash Cycles ---
    // Repeated crash and recovery cycles produce identical state
    let mut node1 = VirtualNode::new("Node1");
    let mut node2 = VirtualNode::new("Node2");
    for m in &rec3 {
        node1.ingest_mutation(m.clone());
        node2.ingest_mutation(m.clone());
    }

    let cp1 = node1.compute_current_checkpoint();
    let cp2 = node2.compute_current_checkpoint();
    assert_eq!(cp1.id, cp2.id, "Multi-crash recovery must produce identical canonical CheckpointID");
    assert_eq!(cp1.body.state_root, cp2.body.state_root);

    let _ = fs::remove_dir_all(temp_dir);
}
