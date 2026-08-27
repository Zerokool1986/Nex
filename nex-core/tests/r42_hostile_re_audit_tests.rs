use std::collections::BTreeMap;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use nex_core::api::NexCoreRuntime;
use nex_core::apps::extensions::NexExtensionHost;
use nex_core::identity::types::{
    KeyType, CapabilityToken, CapabilityProof, OP_READ, OP_WRITE
};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token, verify_capability_chain};
use nex_core::sync::node::VirtualNode;
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::hash::hash_mutation_body;

#[test]
fn test_r42_a_constitutional_boundary_and_lamport_enforcement() {
    let mut node = VirtualNode::new("ConstitutionalGuard");

    // Attempt to inject an extension mutation with an illegal, non-monotonic Lamport clock (e.g. Lamport 99 on Genesis) -> Must be REJECTED
    let illegal_body = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 99, // Non-zero genesis Lamport!
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW { id: [0x88; 32], value: b"MALICIOUS_GENESIS".to_vec() },
    };
    let illegal_id = hash_mutation_body(&illegal_body);
    let disposition = node.ingest_mutation(Mutation { id: illegal_id, body: illegal_body });

    match disposition {
        nex_core::sync::types::IngressDisposition::Rejected(_) => {
            // Correctly rejected by NEX-01/02 causal rules
        },
        _ => panic!("Illegal Lamport genesis mutation must be strictly rejected"),
    }
}

#[test]
fn test_r42_b_capability_confusion_and_amplification_rejection() {
    let mut csprng = OsRng;
    let user_key = SigningKey::generate(&mut csprng);
    let user_pubkey = user_key.verifying_key().to_bytes();
    let user_actor = derive_actor_id(KeyType::Ed25519, &user_pubkey);

    let app_a_key = SigningKey::generate(&mut csprng);
    let app_a_pubkey = app_a_key.verifying_key().to_bytes();
    let app_a_actor = derive_actor_id(KeyType::Ed25519, &app_a_pubkey);

    let app_b_key = SigningKey::generate(&mut csprng);
    let app_b_pubkey = app_b_key.verifying_key().to_bytes();
    let app_b_actor = derive_actor_id(KeyType::Ed25519, &app_b_pubkey);

    let ns_a = [0xAA; 32];
    let ns_b = [0xBB; 32];

    // User grants App A READ permission on Namespace A
    let token_a = CapabilityToken {
        issuer: user_actor,
        subject: app_a_actor,
        allowed_operations: OP_READ,
        namespace: ns_a,
        object_id: None,
        not_before_epoch: 0,
        expires_at_epoch: 10,
        delegation_depth: 1,
        parent_token_hash: None,
    };
    let token_a_hash = hash_capability_token(&token_a);
    let sig_a = user_key.sign(&token_a_hash);
    let proof_a = CapabilityProof {
        token: token_a,
        issuer_pubkey: Some(user_pubkey.to_vec()),
        signature: sig_a.to_bytes().to_vec(),
        parent_proof: None,
    };

    let empty_revocations = BTreeMap::new();

    // 1. Confused Deputy: App A tries to use Proof A to access Namespace B -> REJECT
    let res_ns = verify_capability_chain(
        &proof_a,
        OP_READ,
        &ns_b,
        None,
        1,
        &empty_revocations,
        &user_actor,
    );
    assert!(res_ns.is_err(), "Proof bound to Namespace A cannot be used on Namespace B");

    // 2. Confused Deputy: App B tries to present App A's proof claiming to be grantee -> REJECT
    let res_grantee = verify_capability_chain(
        &proof_a,
        OP_READ,
        &ns_a,
        None,
        1,
        &empty_revocations,
        &app_b_actor, // Wrong root/actor check
    );
    assert!(res_grantee.is_err(), "Mismatched grantee actor must be rejected");

    // 3. Permission Amplification: App A tries to sign a child proof granting OP_WRITE -> REJECT
    let amplified_child = CapabilityToken {
        issuer: app_a_actor,
        subject: app_a_actor,
        allowed_operations: OP_WRITE, // Amplified beyond OP_READ
        namespace: ns_a,
        object_id: None,
        not_before_epoch: 0,
        expires_at_epoch: 10,
        delegation_depth: 0,
        parent_token_hash: Some(token_a_hash),
    };
    let child_hash = hash_capability_token(&amplified_child);
    let sig_child = app_a_key.sign(&child_hash);
    let proof_child = CapabilityProof {
        token: amplified_child,
        issuer_pubkey: Some(app_a_pubkey.to_vec()),
        signature: sig_child.to_bytes().to_vec(),
        parent_proof: Some(Box::new(proof_a)),
    };
    let res_amp = verify_capability_chain(
        &proof_child,
        OP_WRITE,
        &ns_a,
        None,
        1,
        &empty_revocations,
        &user_actor,
    );
    assert!(res_amp.is_err(), "Child token cannot amplify parent permissions");
}

#[test]
fn test_r42_c_multi_tenant_namespace_disjointness() {
    let mut csprng = OsRng;
    let user1_key = SigningKey::generate(&mut csprng);
    let user1_pubkey = user1_key.verifying_key().to_bytes();
    let user1_actor = derive_actor_id(KeyType::Ed25519, &user1_pubkey);

    let user2_key = SigningKey::generate(&mut csprng);
    let user2_pubkey = user2_key.verifying_key().to_bytes();
    let user2_actor = derive_actor_id(KeyType::Ed25519, &user2_pubkey);

    let host1 = NexExtensionHost::new(user1_actor, NexCoreRuntime::new(user1_key, None));
    let host2 = NexExtensionHost::new(user2_actor, NexCoreRuntime::new(user2_key, None));

    // Both users install the exact same application ID
    let app_id = "com.sovereign.todo";
    let ns1 = host1.derive_app_namespace(app_id);
    let ns2 = host2.derive_app_namespace(app_id);

    // Namespaces MUST be completely disjoint
    assert_ne!(ns1, ns2, "Distinct users must yield distinct, isolated application namespaces");
}

#[test]
fn test_r42_e_untrusted_extension_object_wire_replication() {
    let mut node_a = VirtualNode::new("ProducerNode");
    let mut node_b = VirtualNode::new("ConsumerNodeWithoutAppInstalled");

    // Producer creates custom extension object payload (0x1102 Custom Note)
    let body = MutationBody {
        author: [0u8; 32],
        parents: vec![],
        lamport: 0,
        epoch: 0,
        is_resurrect: false,
        payload: CrdtPayload::AddLWW {
            id: [0x42; 32],
            value: b"{\"custom_extension_schema_json\":true}".to_vec(),
        },
    };
    let m_id = hash_mutation_body(&body);
    let mutation = Mutation { id: m_id, body };

    // Ingest on Producer
    node_a.ingest_mutation(mutation.clone());

    // Ingest on Consumer (which has zero knowledge of the extension)
    let disposition = node_b.ingest_mutation(mutation);
    match disposition {
        nex_core::sync::types::IngressDisposition::AdmittedApplied(_) => {
            // Consumer safely ingested and replicated the object without needing third-party code
        },
        _ => panic!("Untrusted extension object must be safely replicated without requiring app installation"),
    }

    // Assert identical state roots
    let cp_a = node_a.compute_current_checkpoint();
    let cp_b = node_b.compute_current_checkpoint();
    assert_eq!(cp_a.body.state_root, cp_b.body.state_root, "State roots must be byte-for-byte identical");
}

#[test]
fn test_r42_i_cross_gate_adversarial_mutation_fuzzing() {
    let mut node = VirtualNode::new("FuzzNode");
    let mut last_id = None;

    // Simulate 100 heterogeneous cross-gate mutations spanning Drive, Photos, Chat, Communities, Extensions
    for i in 0..100 {
        let parents = last_id.map(|id| vec![id]).unwrap_or_default();
        let payload = match i % 5 {
            0 => CrdtPayload::AddLWW { id: [i as u8; 32], value: vec![0x01; 64] }, // Drive Inode
            1 => CrdtPayload::AddLWW { id: [0x20 + i as u8; 32], value: vec![0x02; 64] }, // Photo Media
            2 => CrdtPayload::AddLWW { id: [0x40 + i as u8; 32], value: vec![0x03; 64] }, // Chat Message
            3 => CrdtPayload::AddLWW { id: [0x60 + i as u8; 32], value: vec![0x04; 64] }, // Extension Object
            _ => CrdtPayload::Tombstone { id: [i as u8; 32] }, // Tombstone
        };

        let body = MutationBody {
            author: [0u8; 32],
            parents,
            lamport: i as u64,
            epoch: 0,
            is_resurrect: false,
            payload,
        };
        let m_id = hash_mutation_body(&body);
        let disposition = node.ingest_mutation(Mutation { id: m_id, body });
        assert!(matches!(disposition, nex_core::sync::types::IngressDisposition::AdmittedApplied(_)));
        last_id = Some(m_id);
    }

    let cp = node.compute_current_checkpoint();
    assert_ne!(cp.body.state_root, [0u8; 32]);
    assert_ne!(cp.body.causal_root, [0u8; 32]);
}
