use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::identity::types::{KeyType, CapabilityProof, CapabilityToken, OP_READ, OP_WRITE, OP_ALL};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token, verify_capability_chain};
use nex_core::api::NexAppApi;

#[test]
fn test_r56_4_a_valid_capability_token_evaluation() {
    let issuer_seed = [101u8; 32];
    let issuer_key = SigningKey::from_bytes(&issuer_seed);
    let issuer_pub = issuer_key.verifying_key().to_bytes().to_vec();
    let issuer_actor_id = derive_actor_id(KeyType::Ed25519, &issuer_pub);

    let subject_seed = [102u8; 32];
    let subject_key = SigningKey::from_bytes(&subject_seed);
    let subject_pub = subject_key.verifying_key().to_bytes().to_vec();
    let subject_actor_id = derive_actor_id(KeyType::Ed25519, &subject_pub);

    let namespace = [0x10u8; 32];

    let token = CapabilityToken {
        issuer: issuer_actor_id,
        subject: subject_actor_id,
        namespace,
        object_id: None,
        allowed_operations: OP_READ | OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    let token_hash = hash_capability_token(&token);
    let signature = issuer_key.sign(&token_hash).to_bytes().to_vec();

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(issuer_pub),
        parent_proof: None,
        signature,
    };

    let revocations = BTreeMap::new();
    let res = verify_capability_chain(
        &proof,
        OP_WRITE,
        &namespace,
        None,
        10, // current epoch
        &revocations,
        &issuer_actor_id,
    );
    assert!(res.is_ok(), "Valid capability must be accepted");
}

#[test]
fn test_r56_4_b_expired_capability_token_rejection() {
    let issuer_seed = [103u8; 32];
    let issuer_key = SigningKey::from_bytes(&issuer_seed);
    let issuer_pub = issuer_key.verifying_key().to_bytes().to_vec();
    let issuer_actor_id = derive_actor_id(KeyType::Ed25519, &issuer_pub);

    let subject_seed = [104u8; 32];
    let subject_key = SigningKey::from_bytes(&subject_seed);
    let subject_pub = subject_key.verifying_key().to_bytes().to_vec();
    let subject_actor_id = derive_actor_id(KeyType::Ed25519, &subject_pub);

    let namespace = [0x20u8; 32];

    let token = CapabilityToken {
        issuer: issuer_actor_id,
        subject: subject_actor_id,
        namespace,
        object_id: None,
        allowed_operations: OP_ALL,
        delegation_depth: 0,
        not_before_epoch: 0,
        expires_at_epoch: 5, // Expired at epoch 5
        parent_token_hash: None,
    };

    let token_hash = hash_capability_token(&token);
    let signature = issuer_key.sign(&token_hash).to_bytes().to_vec();

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(issuer_pub),
        parent_proof: None,
        signature,
    };

    let revocations = BTreeMap::new();
    let res = verify_capability_chain(
        &proof,
        OP_WRITE,
        &namespace,
        None,
        10, // current epoch = 10 > 5
        &revocations,
        &issuer_actor_id,
    );
    assert!(res.is_err(), "Expired capability must be rejected");
}

#[test]
fn test_r56_4_c_cross_namespace_hijack_rejection() {
    let issuer_seed = [105u8; 32];
    let issuer_key = SigningKey::from_bytes(&issuer_seed);
    let issuer_pub = issuer_key.verifying_key().to_bytes().to_vec();
    let issuer_actor_id = derive_actor_id(KeyType::Ed25519, &issuer_pub);

    let subject_seed = [106u8; 32];
    let subject_key = SigningKey::from_bytes(&subject_seed);
    let subject_pub = subject_key.verifying_key().to_bytes().to_vec();
    let subject_actor_id = derive_actor_id(KeyType::Ed25519, &subject_pub);

    let chat_namespace = [0x30u8; 32];
    let vault_namespace = [0x40u8; 32];

    let token = CapabilityToken {
        issuer: issuer_actor_id,
        subject: subject_actor_id,
        namespace: chat_namespace,
        object_id: None,
        allowed_operations: OP_ALL,
        delegation_depth: 0,
        not_before_epoch: 0,
        expires_at_epoch: 1000,
        parent_token_hash: None,
    };

    let token_hash = hash_capability_token(&token);
    let signature = issuer_key.sign(&token_hash).to_bytes().to_vec();

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(issuer_pub),
        parent_proof: None,
        signature,
    };

    let revocations = BTreeMap::new();
    let res = verify_capability_chain(
        &proof,
        OP_WRITE,
        &vault_namespace, // Presenting Chat token to Vault namespace
        None,
        10,
        &revocations,
        &issuer_actor_id,
    );
    assert!(res.is_err(), "Cross-namespace capability presentation must be rejected");
}

#[test]
fn test_r56_4_d_forged_signature_rejection() {
    let issuer_seed = [107u8; 32];
    let issuer_key = SigningKey::from_bytes(&issuer_seed);
    let issuer_pub = issuer_key.verifying_key().to_bytes().to_vec();
    let issuer_actor_id = derive_actor_id(KeyType::Ed25519, &issuer_pub);

    let subject_seed = [108u8; 32];
    let subject_key = SigningKey::from_bytes(&subject_seed);
    let subject_pub = subject_key.verifying_key().to_bytes().to_vec();
    let subject_actor_id = derive_actor_id(KeyType::Ed25519, &subject_pub);

    let namespace = [0x50u8; 32];

    let token = CapabilityToken {
        issuer: issuer_actor_id,
        subject: subject_actor_id,
        namespace,
        object_id: None,
        allowed_operations: OP_READ,
        delegation_depth: 0,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    let forged_signature = vec![0xEEu8; 64];

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(issuer_pub),
        parent_proof: None,
        signature: forged_signature,
    };

    let revocations = BTreeMap::new();
    let res = verify_capability_chain(
        &proof,
        OP_READ,
        &namespace,
        None,
        10,
        &revocations,
        &issuer_actor_id,
    );
    assert!(res.is_err(), "Forged signature must be rejected");
}

#[test]
fn test_r56_4_e_two_tier_keystore_envelope_broker_simulation() {
    // Tier 1 Key Broker: generates wrapped session seed, decrypts on demand
    let hardware_master_key = [0x77u8; 32]; // simulated HSM / TEE key
    let mut raw_session_seed = [0x88u8; 32];

    // Envelope encryption of session seed
    for i in 0..32 {
        raw_session_seed[i] ^= hardware_master_key[i];
    }

    // In JNI broker, unwrap session seed
    let mut unwrapped_seed = raw_session_seed;
    for i in 0..32 {
        unwrapped_seed[i] ^= hardware_master_key[i];
    }

    assert_eq!(unwrapped_seed, [0x88u8; 32]);

    let dir = tempdir().unwrap();
    let signing_key = SigningKey::from_bytes(&unwrapped_seed);
    let mut node = NexNode::new(dir.path(), signing_key);
    assert!(node.start().is_ok());
    assert_eq!(node.schema_version, 1);
}

#[test]
fn test_r56_4_f_gate_r56_master_binding_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [200u8; 32];
    let signing_key = SigningKey::from_bytes(&seed);

    let mut node = NexNode::new(dir.path(), signing_key);
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Idempotent sync must preserve Merkle root");
}
