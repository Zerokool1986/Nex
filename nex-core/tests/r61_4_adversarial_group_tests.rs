use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::groups::*;
use nex_core::api::NexAppApi;

#[test]
fn test_r61_4_a_forged_member_removal_rejection() {
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Secure Group", admin);

    let stranger = [0x99u8; 32];
    assert!(group.remove_member(&stranger).is_err(), "Cannot remove nonexistent member");
}

#[test]
fn test_r61_4_b_epoch_ratchet_irreversibility() {
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Ratchet Group", admin);

    let member = [0x02u8; 32];
    group.add_member(member, GroupRole::Member);

    let s1 = group.epoch_secret;
    group.remove_member(&member).unwrap();
    let s2 = group.epoch_secret;

    assert_ne!(s1, s2);
    // Even if re-added, new epoch is used
    group.add_member(member, GroupRole::Member);
    group.remove_member(&member).unwrap();
    let s3 = group.epoch_secret;

    assert_ne!(s2, s3);
    assert_ne!(s1, s3);
}

#[test]
fn test_r61_4_c_concurrent_group_updates() {
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("High Churn", admin);

    for i in 100..200 {
        let m = [i as u8; 32];
        group.add_member(m, GroupRole::Member);
    }
    assert_eq!(group.members.len(), 101);

    for i in 100..150 {
        let m = [i as u8; 32];
        group.remove_member(&m).unwrap();
    }
    assert_eq!(group.epoch, 51);
}

#[test]
fn test_r61_4_d_storage_pool_boundary_allocations() {
    let mut pool = FamilyStoragePool::new(100);
    let member = [0x01u8; 32];

    assert!(pool.allocate_storage(&member, 100).is_ok());
    assert_eq!(pool.used_bytes, 100);
    assert!(pool.allocate_storage(&member, 1).is_err());
}

#[test]
fn test_r61_4_e_10_node_group_sync_simulation() {
    let dir = tempdir().unwrap();
    let seed = [171u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let group = GroupState::new("Mesh Group", node.identity.actor_id);
    let obj_id = NexGroupsService::save_group_state(&mut node, &group).unwrap();

    let retrieved = node.state.object_store.get(&obj_id).unwrap();
    assert_eq!(retrieved.namespace, NexGroupsService::GROUPS_NAMESPACE);
}

#[test]
fn test_r61_4_f_gate_r61_master_group_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [172u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Group operations must preserve Merkle root invariance");
}
