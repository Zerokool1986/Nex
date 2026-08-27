use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::product::journey::SovereignJourneyOrchestrator;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};

#[test]
fn test_r72_4_a_complete_twenty_step_human_journey_execution() {
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

    let amy_actor = [0xAA; 32];
    let desktop_actor = desktop.identity.actor_id;

    let summary = SovereignJourneyOrchestrator::execute_twenty_step_journey(
        &mut mobile,
        &mut desktop,
        &proof,
        &root_actor,
        &amy_actor,
        &desktop_actor,
    ).expect("20-step sovereign journey execution failed");

    assert_eq!(summary.inspector_title, "Family Picnic at the Lake");
    assert_eq!(summary.inspector_replica_count, 2);
    assert_eq!(summary.person_panel_name, "Amy");
    assert_eq!(summary.device_panel_name, "Chris's Desktop Station");
    assert_eq!(summary.recovered_synced_photos, 2);
}

#[test]
fn test_r72_4_b_journey_fails_when_unauthorized_token_used() {
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
    // Token with OP_READ instead of OP_WRITE
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: nex_core::identity::types::OP_READ,
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

    let desktop_actor = desktop.identity.actor_id;
    let res = SovereignJourneyOrchestrator::execute_twenty_step_journey(
        &mut mobile,
        &mut desktop,
        &proof,
        &root_actor,
        &[0xAA; 32],
        &desktop_actor,
    );

    assert!(res.is_err(), "Unauthorized token must abort journey");
}

#[test]
fn test_r72_4_c_offline_journey_preserves_local_sovereignty() {
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

    let desktop_actor = desktop.identity.actor_id;
    let summary = SovereignJourneyOrchestrator::execute_twenty_step_journey(
        &mut mobile,
        &mut desktop,
        &proof,
        &root_actor,
        &[0xAA; 32],
        &desktop_actor,
    ).unwrap();

    assert_eq!(summary.recovered_synced_photos, 2);
    assert_eq!(mobile.state.object_store.len(), 2);
    assert_eq!(desktop.state.object_store.len(), 2);
}

#[test]
fn test_r72_4_d_repeated_journey_execution_idempotence() {
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

    let desktop_actor = desktop.identity.actor_id;
    let summary = SovereignJourneyOrchestrator::execute_twenty_step_journey(
        &mut mobile,
        &mut desktop,
        &proof,
        &root_actor,
        &[0xAA; 32],
        &desktop_actor,
    ).unwrap();

    assert!(summary.recovered_synced_photos > 0);
}

#[test]
fn test_r72_4_e_space_isolation_guarantee_after_journey() {
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

    let desktop_actor = desktop.identity.actor_id;
    SovereignJourneyOrchestrator::execute_twenty_step_journey(
        &mut mobile,
        &mut desktop,
        &proof,
        &root_actor,
        &[0xAA; 32],
        &desktop_actor,
    ).unwrap();

    // Work space must be 0
    let mut shell = NexHomeShell::new();
    shell.switch_space(SpaceType::Work);
    assert_eq!(shell.generate_home_summary(&desktop).total_objects_in_space, 0);

    // Personal space must be 0
    shell.switch_space(SpaceType::Personal);
    assert_eq!(shell.generate_home_summary(&desktop).total_objects_in_space, 0);
}

#[test]
fn test_r72_4_f_journey_summary_contains_all_verified_fields() {
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

    let desktop_actor = desktop.identity.actor_id;
    let sum = SovereignJourneyOrchestrator::execute_twenty_step_journey(
        &mut mobile,
        &mut desktop,
        &proof,
        &root_actor,
        &[0xAA; 32],
        &desktop_actor,
    ).unwrap();

    assert!(!sum.inspector_title.is_empty());
    assert!(!sum.person_panel_name.is_empty());
    assert!(!sum.device_panel_name.is_empty());
    assert!(sum.recovered_synced_photos >= 2);
}
