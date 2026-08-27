use std::collections::BTreeMap;
use nex_core::identity::types::{
    KeyType, CapabilityToken, CapabilityProof, DeviceCertificate, AuthorizationError,
    OP_REGISTER_LWW, OP_SET_ADD, OP_SET_REMOVE, OP_ALL
};
use nex_core::identity::verifier::{
    derive_actor_id, hash_capability_token, verify_capability_chain, verify_device_certificate
};

#[test]
fn test_r22_a_identity_determinism_and_self_certification() {
    let pk_alice = [0x01u8; 32];
    let pk_bob = [0x02u8; 32];

    let id_alice_ed = derive_actor_id(KeyType::Ed25519, &pk_alice);
    let id_alice_ed_2 = derive_actor_id(KeyType::Ed25519, &pk_alice);
    let id_alice_secp = derive_actor_id(KeyType::Secp256k1, &pk_alice);
    let id_bob_ed = derive_actor_id(KeyType::Ed25519, &pk_bob);

    assert_eq!(id_alice_ed, id_alice_ed_2, "R22-A: ActorID derivation must be strictly deterministic");
    assert_ne!(id_alice_ed, id_alice_secp, "R22-A: ActorID must be key-type sensitive");
    assert_ne!(id_alice_ed, id_bob_ed, "R22-A: ActorID must be public-key sensitive");
}

#[test]
fn test_r22_b_signature_authenticity_and_root_authorization() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let namespace = [0xAA; 32];

    let root_token = CapabilityToken {
        issuer: alice,
        subject: alice,
        namespace,
        object_id: None,
        allowed_operations: OP_ALL,
        delegation_depth: 3,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    // Valid proof with signature
    let valid_proof = CapabilityProof {
        token: root_token.clone(),
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0xEE; 64],
    };

    let empty_revocations = BTreeMap::new();
    let res = verify_capability_chain(
        &valid_proof,
        OP_REGISTER_LWW,
        &namespace,
        None,
        10,
        &empty_revocations,
        &alice,
    );
    assert_eq!(res, Ok(alice), "R22-B: Valid root capability must verify successfully");

    // Invalid proof with empty signature
    let mut invalid_sig_proof = valid_proof.clone();
    invalid_sig_proof.signature.clear();
    let res_err = verify_capability_chain(
        &invalid_sig_proof,
        OP_REGISTER_LWW,
        &namespace,
        None,
        10,
        &empty_revocations,
        &alice,
    );
    assert_eq!(res_err, Err(AuthorizationError::SignatureInvalid), "R22-B: Empty signature must be rejected");
}

#[test]
fn test_r22_d_capability_attenuation_and_chained_delegation() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let charlie = derive_actor_id(KeyType::Ed25519, &[0x03; 32]);
    let dave = derive_actor_id(KeyType::Ed25519, &[0x04; 32]);

    let namespace_photos = [0x10; 32];
    let empty_revocations = BTreeMap::new();

    // 1. Alice (Root) grants Bob full namespace operations on /Photos (Depth 2)
    let alice_token = CapabilityToken {
        issuer: alice,
        subject: bob,
        namespace: namespace_photos,
        object_id: None,
        allowed_operations: OP_REGISTER_LWW | OP_SET_ADD | OP_SET_REMOVE,
        delegation_depth: 2,
        not_before_epoch: 0,
        expires_at_epoch: 50,
        parent_token_hash: None,
    };
    let alice_proof = CapabilityProof {
        token: alice_token.clone(),
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x11; 64],
    };

    // 2. Bob delegates to Charlie (Attenuated: Read/Write only, Depth 1)
    let bob_token = CapabilityToken {
        issuer: bob,
        subject: charlie,
        namespace: namespace_photos,
        object_id: None,
        allowed_operations: OP_REGISTER_LWW | OP_SET_ADD, // Attenuated: Removed SET_REMOVE
        delegation_depth: 1, // Decremented depth
        not_before_epoch: 5,
        expires_at_epoch: 40,
        parent_token_hash: Some(hash_capability_token(&alice_token)),
    };
    let bob_proof = CapabilityProof {
        token: bob_token.clone(),
        issuer_pubkey: None,
        parent_proof: Some(Box::new(alice_proof.clone())),
        signature: vec![0x22; 64],
    };

    // Verify Charlie's delegated capability for OP_SET_ADD
    let res_charlie = verify_capability_chain(
        &bob_proof,
        OP_SET_ADD,
        &namespace_photos,
        None,
        15,
        &empty_revocations,
        &alice,
    );
    assert_eq!(res_charlie, Ok(charlie), "R22-D: Attenuated delegation chain must verify for authorized operation");

    // Charlie attempts an unauthorized operation (SET_REMOVE) which was attenuated away by Bob
    let res_escalation = verify_capability_chain(
        &bob_proof,
        OP_SET_REMOVE,
        &namespace_photos,
        None,
        15,
        &empty_revocations,
        &alice,
    );
    assert!(matches!(res_escalation, Err(AuthorizationError::UnauthorizedOperation { .. })), "R22-D: Attenuated operation must be rejected");

    // 3. Charlie delegates to Dave (Terminal: Depth 0)
    let charlie_token = CapabilityToken {
        issuer: charlie,
        subject: dave,
        namespace: namespace_photos,
        object_id: None,
        allowed_operations: OP_SET_ADD,
        delegation_depth: 0, // Terminal depth
        not_before_epoch: 10,
        expires_at_epoch: 30,
        parent_token_hash: Some(hash_capability_token(&bob_token)),
    };
    let charlie_proof = CapabilityProof {
        token: charlie_token.clone(),
        issuer_pubkey: None,
        parent_proof: Some(Box::new(bob_proof.clone())),
        signature: vec![0x33; 64],
    };

    let res_dave = verify_capability_chain(
        &charlie_proof,
        OP_SET_ADD,
        &namespace_photos,
        None,
        20,
        &empty_revocations,
        &alice,
    );
    assert_eq!(res_dave, Ok(dave), "R22-D: Dave must be authorized at terminal depth 0");

    // 4. Dave attempts to re-delegate (Depth Exhaustion violation)
    let eve = derive_actor_id(KeyType::Ed25519, &[0x05; 32]);
    let dave_token = CapabilityToken {
        issuer: dave,
        subject: eve,
        namespace: namespace_photos,
        object_id: None,
        allowed_operations: OP_SET_ADD,
        delegation_depth: 0,
        not_before_epoch: 10,
        expires_at_epoch: 25,
        parent_token_hash: Some(hash_capability_token(&charlie_token)),
    };
    let dave_proof = CapabilityProof {
        token: dave_token,
        issuer_pubkey: None,
        parent_proof: Some(Box::new(charlie_proof.clone())),
        signature: vec![0x44; 64],
    };

    let res_depth_exceeded = verify_capability_chain(
        &dave_proof,
        OP_SET_ADD,
        &namespace_photos,
        None,
        20,
        &empty_revocations,
        &alice,
    );
    assert_eq!(res_depth_exceeded, Err(AuthorizationError::DelegationDepthExceeded), "R22-D: Re-delegating from depth 0 must be rejected");
}

#[test]
fn test_r22_e_namespace_and_object_scope_bounding() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);

    let namespace_a = [0xAA; 32];
    let namespace_b = [0xBB; 32];
    let obj_1 = [0x01; 32];
    let obj_2 = [0x02; 32];

    let token = CapabilityToken {
        issuer: alice,
        subject: bob,
        namespace: namespace_a,
        object_id: Some(obj_1), // Bound strictly to obj_1
        allowed_operations: OP_ALL,
        delegation_depth: 1,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let proof = CapabilityProof {
        token,
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x99; 64],
    };

    let empty_revocations = BTreeMap::new();

    // 1. Correct namespace and correct object -> Ok
    let res_ok = verify_capability_chain(&proof, OP_SET_ADD, &namespace_a, Some(&obj_1), 10, &empty_revocations, &alice);
    assert_eq!(res_ok, Ok(bob));

    // 2. Wrong namespace -> NamespaceMismatch
    let res_ns_err = verify_capability_chain(&proof, OP_SET_ADD, &namespace_b, Some(&obj_1), 10, &empty_revocations, &alice);
    assert_eq!(res_ns_err, Err(AuthorizationError::NamespaceMismatch));

    // 3. Wrong object -> ObjectMismatch
    let res_obj_err = verify_capability_chain(&proof, OP_SET_ADD, &namespace_a, Some(&obj_2), 10, &empty_revocations, &alice);
    assert_eq!(res_obj_err, Err(AuthorizationError::ObjectMismatch));
}

#[test]
fn test_r22_f_revocation_and_epoch_fencing() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let namespace = [0xCC; 32];

    let token = CapabilityToken {
        issuer: alice,
        subject: bob,
        namespace,
        object_id: None,
        allowed_operations: OP_ALL,
        delegation_depth: 1,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x77; 64],
    };

    // Revocation published at epoch 20
    let mut active_revocations = BTreeMap::new();
    active_revocations.insert(token_hash, 20u64);

    // 1. Presentation at epoch 19 (Historical / Prior to Revocation) -> Valid
    let res_epoch_19 = verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 19, &active_revocations, &alice);
    assert_eq!(res_epoch_19, Ok(bob), "R22-F: Historical capability valid prior to revocation epoch");

    // 2. Presentation at epoch 20 (Revocation epoch) -> RevokedCapability
    let res_epoch_20 = verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 20, &active_revocations, &alice);
    assert!(matches!(res_epoch_20, Err(AuthorizationError::RevokedCapability { .. })), "R22-F: Capability must be rejected at revocation epoch");

    // 3. Presentation at epoch 25 (Post-Revocation) -> RevokedCapability
    let res_epoch_25 = verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 25, &active_revocations, &alice);
    assert!(matches!(res_epoch_25, Err(AuthorizationError::RevokedCapability { .. })), "R22-F: Capability must be rejected post-revocation");
}

#[test]
fn test_r22_h_multi_device_authority_and_rotation() {
    let master_alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let phone_alice = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let laptop_alice = derive_actor_id(KeyType::Ed25519, &[0x03; 32]);

    let phone_cert = DeviceCertificate {
        master_actor_id: master_alice,
        device_actor_id: phone_alice,
        not_before_epoch: 0,
        expires_at_epoch: 50,
        master_pubkey: None,
        signature: vec![0xAA; 64],
    };

    let laptop_cert = DeviceCertificate {
        master_actor_id: master_alice,
        device_actor_id: laptop_alice,
        not_before_epoch: 0,
        expires_at_epoch: 50,
        master_pubkey: None,
        signature: vec![0xBB; 64],
    };

    // Both device certificates verify under Master Alice
    assert_eq!(verify_device_certificate(&phone_cert, &master_alice, 10), Ok(()));
    assert_eq!(verify_device_certificate(&laptop_cert, &master_alice, 10), Ok(()));

    // Expired device cert
    assert!(matches!(
        verify_device_certificate(&phone_cert, &master_alice, 51),
        Err(AuthorizationError::ExpiredCapability { .. })
    ));
}

#[test]
fn test_r22_i_byzantine_cyclic_delegation_rejection() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let namespace = [0xDD; 32];

    let mut token_1 = CapabilityToken {
        issuer: alice,
        subject: bob,
        namespace,
        object_id: None,
        allowed_operations: OP_ALL,
        delegation_depth: 2,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    let mut token_2 = CapabilityToken {
        issuer: bob,
        subject: alice,
        namespace,
        object_id: None,
        allowed_operations: OP_ALL,
        delegation_depth: 1,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };

    // Link parent token hashes mutually
    let token_1_base_hash = hash_capability_token(&token_1);
    token_2.parent_token_hash = Some(token_1_base_hash);
    let token_2_hash = hash_capability_token(&token_2);
    token_1.parent_token_hash = Some(token_2_hash);
    let token_1_hash = hash_capability_token(&token_1);
    token_2.parent_token_hash = Some(token_1_hash);

    let mut proof_2 = CapabilityProof {
        token: token_2.clone(),
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x22; 64],
    };

    let mut proof_1 = CapabilityProof {
        token: token_1.clone(),
        issuer_pubkey: None,
        parent_proof: Some(Box::new(proof_2.clone())),
        signature: vec![0x11; 64],
    };

    // Complete the cycle: proof_2 -> proof_1 -> proof_2
    proof_2.parent_proof = Some(Box::new(proof_1.clone()));
    proof_1.parent_proof = Some(Box::new(proof_2));

    let empty_revocations = BTreeMap::new();
    let res = verify_capability_chain(&proof_1, OP_ALL, &namespace, None, 10, &empty_revocations, &alice);
    assert!(
        matches!(res, Err(AuthorizationError::CyclicDelegationDetected) | Err(AuthorizationError::ParentAttenuationViolation(_))),
        "R22-I: Cyclic delegation must be detected and rejected"
    );
}

#[test]
fn test_r22_c_authority_domain_isolation() {
    let master_alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let phone_alice = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let namespace = [0xAA; 32];

    // Device attempts to self-issue a Root Capability claim claiming to be Master
    let forged_root_token = CapabilityToken {
        issuer: phone_alice, // Non-master attempting root issuance
        subject: phone_alice,
        namespace,
        object_id: None,
        allowed_operations: OP_ALL,
        delegation_depth: 3,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let forged_proof = CapabilityProof {
        token: forged_root_token,
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x99; 64],
    };

    let empty_revocations = BTreeMap::new();
    let res = verify_capability_chain(
        &forged_proof,
        OP_REGISTER_LWW,
        &namespace,
        None,
        10,
        &empty_revocations,
        &master_alice,
    );
    assert_eq!(res, Err(AuthorizationError::RootIssuerMismatch), "R22-C: Non-master actor cannot issue root capability");
}

#[test]
fn test_r22_g_replay_and_stale_authority_resistance() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let namespace = [0xAA; 32];

    let token = CapabilityToken {
        issuer: alice,
        subject: bob,
        namespace,
        object_id: None,
        allowed_operations: OP_REGISTER_LWW,
        delegation_depth: 1,
        not_before_epoch: 10,
        expires_at_epoch: 20,
        parent_token_hash: None,
    };
    let proof = CapabilityProof {
        token,
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x11; 64],
    };

    let empty_revocations = BTreeMap::new();

    // 1. Premature presentation (not yet valid)
    let res_early = verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 9, &empty_revocations, &alice);
    assert!(matches!(res_early, Err(AuthorizationError::NotYetValid { .. })), "R22-G: Premature capability presentation must fail");

    // 2. Valid presentation
    let res_valid = verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 15, &empty_revocations, &alice);
    assert_eq!(res_valid, Ok(bob));

    // 3. Stale presentation (expired)
    let res_stale = verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 21, &empty_revocations, &alice);
    assert!(matches!(res_stale, Err(AuthorizationError::ExpiredCapability { .. })), "R22-G: Expired capability presentation must fail");
}

#[test]
fn test_r22_j_cross_node_authorization_convergence() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let namespace = [0xEE; 32];

    let token = CapabilityToken {
        issuer: alice,
        subject: bob,
        namespace,
        object_id: None,
        allowed_operations: OP_REGISTER_LWW,
        delegation_depth: 1,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: None,
        parent_proof: None,
        signature: vec![0x33; 64],
    };

    // Node 1 and Node 2 maintain independent revocation tables
    let mut node1_revocations = BTreeMap::new();
    let mut node2_revocations = BTreeMap::new();

    // Both start in agreement
    assert_eq!(verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 10, &node1_revocations, &alice), Ok(bob));
    assert_eq!(verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 10, &node2_revocations, &alice), Ok(bob));

    // Node 1 receives a revocation record at epoch 15
    node1_revocations.insert(token_hash, 15u64);

    // Node 2 syncs the revocation record from Node 1
    node2_revocations.insert(token_hash, 15u64);

    // Both converge on identical post-revocation rejection
    assert!(matches!(
        verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 16, &node1_revocations, &alice),
        Err(AuthorizationError::RevokedCapability { .. })
    ));
    assert!(matches!(
        verify_capability_chain(&proof, OP_REGISTER_LWW, &namespace, None, 16, &node2_revocations, &alice),
        Err(AuthorizationError::RevokedCapability { .. })
    ));
}

#[test]
fn test_r22_k_root_key_recovery_and_compromise_containment() {
    let master_alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let compromised_phone = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let new_phone = derive_actor_id(KeyType::Ed25519, &[0x03; 32]);

    let old_cert = DeviceCertificate {
        master_actor_id: master_alice,
        device_actor_id: compromised_phone,
        not_before_epoch: 0,
        expires_at_epoch: 50,
        master_pubkey: None,
        signature: vec![0xAA; 64],
    };

    let new_cert = DeviceCertificate {
        master_actor_id: master_alice,
        device_actor_id: new_phone,
        not_before_epoch: 20,
        expires_at_epoch: 70,
        master_pubkey: None,
        signature: vec![0xBB; 64],
    };

    // Before compromise at epoch 10: old device is valid
    assert_eq!(verify_device_certificate(&old_cert, &master_alice, 10), Ok(()));

    // Compromise occurs: master rotates certificates and expires old cert
    // At epoch 25: old cert is expired or superseded, new cert is active
    let old_cert_expired = DeviceCertificate {
        master_actor_id: master_alice,
        device_actor_id: compromised_phone,
        not_before_epoch: 0,
        expires_at_epoch: 20, // Shortened/revoked by master
        master_pubkey: None,
        signature: vec![0xAA; 64],
    };

    assert!(matches!(
        verify_device_certificate(&old_cert_expired, &master_alice, 25),
        Err(AuthorizationError::ExpiredCapability { .. })
    ));
    assert_eq!(verify_device_certificate(&new_cert, &master_alice, 25), Ok(()));
}
