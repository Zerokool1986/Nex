use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::api::NexAppApi;
use nex_core::runtime::diagnostics::{SubstrateHealthDiagnostics, ProgressiveTier};
use nex_core::object::types::ObjectType;

#[test]
fn test_r71_23_a_sync_state_everyday_tier() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x01; 32]));
    node.start().unwrap();

    let str_val = SubstrateHealthDiagnostics::format_sync_state(&node, ProgressiveTier::Everyday);
    assert_eq!(str_val, "All up to date");
}

#[test]
fn test_r71_23_b_sync_state_informational_tier() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x02; 32]));
    node.start().unwrap();

    node.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"data".to_vec()).unwrap();
    let str_val = SubstrateHealthDiagnostics::format_sync_state(&node, ProgressiveTier::Informational);
    assert!(str_val.contains("1 objects verified"));
}

#[test]
fn test_r71_23_c_sync_state_advanced_tier_contains_hex_root() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x03; 32]));
    node.start().unwrap();

    let str_val = SubstrateHealthDiagnostics::format_sync_state(&node, ProgressiveTier::Advanced);
    assert!(str_val.starts_with("SMT State Root: "));
}

#[test]
fn test_r71_23_d_storage_state_3_tiers() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x04; 32]));
    node.start().unwrap();

    let s_everyday = SubstrateHealthDiagnostics::format_storage_state(&node, ProgressiveTier::Everyday);
    let s_info = SubstrateHealthDiagnostics::format_storage_state(&node, ProgressiveTier::Informational);
    let s_adv = SubstrateHealthDiagnostics::format_storage_state(&node, ProgressiveTier::Advanced);

    assert!(s_everyday.contains("Storage healthy"));
    assert!(s_info.contains("KB used"));
    assert!(s_adv.contains("CAS Bytes"));
}

#[test]
fn test_r71_23_e_identity_state_3_tiers() {
    let root = [0x55; 32];
    let s1 = SubstrateHealthDiagnostics::format_identity_state(&root, 3, ProgressiveTier::Everyday);
    let s2 = SubstrateHealthDiagnostics::format_identity_state(&root, 3, ProgressiveTier::Informational);
    let s3 = SubstrateHealthDiagnostics::format_identity_state(&root, 3, ProgressiveTier::Advanced);

    assert!(s1.contains("Sovereign & Protected"));
    assert!(s2.contains("3 hardware devices authorized"));
    assert!(s3.contains("Root ActorID:"));
}

#[test]
fn test_r71_23_f_storage_diagnostics_tracks_tombstones() {
    let tmp = tempdir().unwrap();
    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&[0x05; 32]));
    node.start().unwrap();

    let id = node.create_object([0x01; 32], ObjectType::Synthetic(1), BTreeMap::new(), b"data".to_vec()).unwrap();
    node.delete_object(id, None).unwrap();

    let s_adv = SubstrateHealthDiagnostics::format_storage_state(&node, ProgressiveTier::Advanced);
    assert!(s_adv.contains("Tombstones: 1"));
}
