use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::product::inspector::UniversalObjectInspector;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::ObjectType;

#[test]
fn test_r72_2_a_universal_inspector_exposes_photos_provenance() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Beach Sunset", vec![0xFF; 2048], 10, &BTreeMap::new(), &root_actor).unwrap();

    let insp = UniversalObjectInspector::inspect(&node, &id, InterfaceComplexity::Standard).unwrap();
    assert_eq!(insp.title, "Beach Sunset");
    assert_eq!(insp.object_type, ObjectType::PhotoMedia);
    assert_eq!(insp.space_name, "Family");
    assert_eq!(insp.byte_size, 2048);
    assert_eq!(insp.byte_size_formatted, "2.0 KB");
    assert_eq!(insp.replica_count, 2);
}

#[test]
fn test_r72_2_b_universal_inspector_exposes_drive_document() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x02u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_create_family_document(&mut node, &proof, "House_Deed.pdf", vec![0xEE; 4096], 10, &BTreeMap::new(), &root_actor).unwrap();

    let insp = UniversalObjectInspector::inspect(&node, &id, InterfaceComplexity::Standard).unwrap();
    assert_eq!(insp.title, "House_Deed.pdf");
    assert_eq!(insp.object_type, ObjectType::DriveInode);
    assert_eq!(insp.byte_size_formatted, "4.0 KB");
}

#[test]
fn test_r72_2_c_advanced_tier_reveals_dag_technical_info() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Tech Photo", vec![0x11; 8192], 10, &BTreeMap::new(), &root_actor).unwrap();

    let insp_simple = UniversalObjectInspector::inspect(&node, &id, InterfaceComplexity::Simple).unwrap();
    assert!(insp_simple.advanced_dag_info.is_none());

    let insp_adv = UniversalObjectInspector::inspect(&node, &id, InterfaceComplexity::Advanced).unwrap();
    assert!(insp_adv.advanced_dag_info.is_some());
    let dag = insp_adv.advanced_dag_info.unwrap();
    assert_eq!(dag.schema_version, 1);
    assert_eq!(dag.cas_chunk_count, 2);
}

#[test]
fn test_r72_2_d_available_capabilities_list_is_complete() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x04u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Cap Photo", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let insp = UniversalObjectInspector::inspect(&node, &id, InterfaceComplexity::Simple).unwrap();
    assert_eq!(insp.available_capabilities.len(), 3);
    assert!(insp.available_capabilities.contains(&"Read".to_string()));
    assert!(insp.available_capabilities.contains(&"Share".to_string()));
    assert!(insp.available_capabilities.contains(&"Delete".to_string()));
}

#[test]
fn test_r72_2_e_missing_object_inspection_returns_error() {
    let tmp = tempdir().unwrap();
    let node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x05u8; 32]));
    let fake_id = [0x99; 32];
    assert!(UniversalObjectInspector::inspect(&node, &fake_id, InterfaceComplexity::Simple).is_err());
}

#[test]
fn test_r72_2_f_shared_peers_and_stored_devices_exposure() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x06u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
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
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Peers Photo", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();

    let insp = UniversalObjectInspector::inspect(&node, &id, InterfaceComplexity::Simple).unwrap();
    assert_eq!(insp.shared_with_peers.len(), 2);
    assert_eq!(insp.stored_on_devices.len(), 2);
}
