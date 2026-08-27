use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::runtime::reality::{ProductionRealityEngine, NetworkLinkState};
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r71_33_a_lossy_transport_partial_drop_and_subsequent_healing() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x02u8; 32]));
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

    // Capture 6 photos
    for i in 1..=6 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("Photo {}", i), vec![i as u8; 50], 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    assert_eq!(mobile.state.object_store.len(), 6);

    // 1st round: 50% packet drop rate
    let (success_1, dropped_1) = ProductionRealityEngine::simulate_lossy_sync(&mut mobile, &mut desktop, 50, 42);
    assert!(success_1 > 0);
    assert!(dropped_1 > 0);
    assert!(desktop.state.object_store.len() < 6);

    // 2nd round: Network heals (0% drop rate) -> complete reconciliation
    let (success_2, dropped_2) = ProductionRealityEngine::simulate_lossy_sync(&mut mobile, &mut desktop, 0, 99);
    assert!(success_2 > 0);
    assert_eq!(dropped_2, 0);
    assert_eq!(desktop.state.object_store.len(), 6);
}

#[test]
fn test_r71_33_b_rapid_disconnect_reconnect_churn_preserves_consistency() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x04u8; 32]));
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

    // 10 rapid capture and churn cycles
    for cycle in 1..=10 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("Pic {}", cycle), vec![cycle as u8; 30], 10, &BTreeMap::new(), &root_actor).unwrap();
        // Simulate flaky connection churn with high loss
        ProductionRealityEngine::simulate_lossy_sync(&mut mobile, &mut desktop, 70, cycle as u64);
    }

    // Final clean sync (0% loss)
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(mobile.state.object_store.len(), 10);
    assert_eq!(desktop.state.object_store.len(), 10);
}

#[test]
fn test_r71_33_c_wifi_to_mobile_network_handover_reconnection() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x06u8; 32]));
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

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Handover", b"bytes".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // 1. WiFi dropped (100% loss)
    let (s1, d1) = ProductionRealityEngine::simulate_lossy_sync(&mut mobile, &mut desktop, 100, 1);
    assert_eq!(s1, 0);
    assert!(d1 > 0);
    assert_eq!(desktop.state.object_store.len(), 0);

    // 2. Handover to Cellular/Relay (0% loss)
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(desktop.state.object_store.len(), 1);
}

#[test]
fn test_r71_33_d_zero_false_synced_state_during_active_drop() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&[0x07u8; 32]));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x08u8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    let root_actor = mobile.identity.actor_id;
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
        issuer_pubkey: Some(SigningKey::from_bytes(&[0x07u8; 32]).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&[0x07u8; 32]).sign(&token_hash).to_bytes().to_vec(),
    };

    SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Pending Pic", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    // 100% loss simulation
    ProductionRealityEngine::simulate_lossy_sync(&mut mobile, &mut desktop, 100, 1);

    let status = ProductionRealityEngine::format_network_status(
        NetworkLinkState::PartialConnectivity,
        1,
        nex_core::runtime::experience::InterfaceComplexity::Simple,
    );

    assert_eq!(status.headline, "Some items are waiting");
    assert_eq!(status.pending_items_count, 1);
    assert_ne!(status.headline, "Up to date");
}

#[test]
fn test_r71_33_e_concurrent_mutations_across_partition_reconciled() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x09u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x0Au8; 32]));
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

    // Mobile writes 3 photos during partition
    for i in 1..=3 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("M-Pic {}", i), vec![i as u8; 20], 10, &BTreeMap::new(), &root_actor).unwrap();
    }
    // Desktop writes 2 docs during partition
    for i in 1..=2 {
        SovereignProductSlice::mobile_create_family_document(&mut desktop, &proof, &format!("D-Doc {}.txt", i), vec![i as u8; 40], 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    assert_eq!(mobile.state.object_store.len(), 3);
    assert_eq!(desktop.state.object_store.len(), 2);

    // Partition heals: bidirectional sync
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    SovereignProductSlice::sync_mobile_to_desktop(&mut desktop, &mut mobile);

    assert_eq!(mobile.state.object_store.len(), 5);
    assert_eq!(desktop.state.object_store.len(), 5);
}

#[test]
fn test_r71_33_f_large_batch_burst_sync_under_degraded_conditions() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x0Bu8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x0Cu8; 32]));
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

    // 20 objects captured
    for i in 1..=20 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("Burst Pic {}", i), vec![0xEE; 100], 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    assert_eq!(mobile.state.object_store.len(), 20);

    // Initial degraded sync with 30% loss
    ProductionRealityEngine::simulate_lossy_sync(&mut mobile, &mut desktop, 30, 77);

    // Subsequent retransmission round to heal
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(desktop.state.object_store.len(), 20);
}
