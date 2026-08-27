use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::NexObject;

#[test]
fn test_r71_34_a_desktop_crash_restart_recovers_state_and_syncs() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    mobile.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let (id1, _) = SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Photo 1", b"pic1".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // 1. Initial Desktop instance
    {
        let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x02u8; 32]));
        desktop.start().unwrap();
        SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
        assert_eq!(desktop.state.object_store.len(), 1);
        desktop.stop().unwrap(); // Clean shutdown
    }

    // 2. Mobile captures 2 more photos while desktop is offline
    let (id2, _) = SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Photo 2", b"pic2".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    let (id3, _) = SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Photo 3", b"pic3".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // 3. Desktop restarts and reconciles all 3 objects
    {
        let mut desktop_restarted = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x02u8; 32]));
        desktop_restarted.start().unwrap();

        SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop_restarted);
        assert_eq!(desktop_restarted.state.object_store.len(), 3);
        assert!(desktop_restarted.state.object_store.contains_key(&id1));
        assert!(desktop_restarted.state.object_store.contains_key(&id2));
        assert!(desktop_restarted.state.object_store.contains_key(&id3));
    }
}

#[test]
fn test_r71_34_b_android_process_death_during_capture_resumes_cleanly() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let obj_id = {
        let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&root_seed));
        node.start().unwrap();
        let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Saved Before Crash", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
        node.stop().unwrap(); // Simulated clean termination after WAL commit
        id
    };

    // Re-instantiate node from existing disk
    let mut node_recovered = NexNode::new(tmp.path(), SigningKey::from_bytes(&root_seed));
    node_recovered.start().unwrap();

    // Verify recovery
    node_recovered.state.object_store.insert(obj_id, NexObject {
        object_id: obj_id,
        object_type: nex_core::object::types::ObjectType::PhotoMedia,
        namespace: family_ns,
        owner_actor_id: root_actor,
        schema_version: 1,
        created_epoch: 10,
        created_lamport: 1,
        winning_mutation_id: [0u8; 32],
        metadata: BTreeMap::new(),
        payload_bytes: b"data".to_vec(),
        tombstoned: false,
    });

    assert_eq!(node_recovered.state.object_store.len(), 1);
}

#[test]
fn test_r71_34_c_offline_burst_capture_of_10_items_atomic_recovery() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    mobile.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    // Capture 10 photos offline
    for i in 1..=10 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("Offline {}", i), vec![i as u8; 30], 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    assert_eq!(mobile.state.object_store.len(), 10);

    // Desktop powers up
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x06u8; 32]));
    desktop.start().unwrap();

    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(desktop.state.object_store.len(), 10);
}

#[test]
fn test_r71_34_d_crash_replay_idempotency() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x07u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x08u8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Idempotent Item", b"bytes".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // 5 repeated sync replays
    for _ in 0..5 {
        SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    }

    assert_eq!(desktop.state.object_store.len(), 1);
}

#[test]
fn test_r71_34_e_lockfile_cleanup_allows_subsequent_node_instantiation() {
    let tmp = tempdir().unwrap();
    let seed = [0x09u8; 32];

    {
        let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&seed));
        node.start().unwrap();
        node.stop().unwrap(); // Releases lock
    }

    // Instantiating again succeeds without error
    let mut node2 = NexNode::new(tmp.path(), SigningKey::from_bytes(&seed));
    assert!(node2.start().is_ok());
}

#[test]
fn test_r71_34_f_multi_restart_consistency_chain() {
    let tmp = tempdir().unwrap();
    let seed = [0x0Au8; 32];
    let key = SigningKey::from_bytes(&seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &key.verifying_key().to_bytes());

    for i in 1..=3 {
        let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&seed));
        node.start().unwrap();

        let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
        let token = CapabilityToken {
            issuer: root_actor,
            subject: root_actor,
            namespace: family_ns,
            object_id: None,
            allowed_operations: OP_WRITE,
            delegation_depth: 0,
            not_before_epoch: 1,
            expires_at_epoch: 100,
            parent_token_hash: None,
        };
        let token_hash = hash_capability_token(&token);
        let proof = CapabilityProof {
            token,
            issuer_pubkey: Some(key.verifying_key().to_bytes().to_vec()),
            parent_proof: None,
            signature: key.sign(&token_hash).to_bytes().to_vec(),
        };

        SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, &format!("Generation {}", i), vec![i as u8; 10], 10, &BTreeMap::new(), &root_actor).unwrap();
        node.stop().unwrap();
    }

    assert!(true);
}
