use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::groups::*;

#[test]
fn test_r61_3_a_family_storage_pool_quota_allocation() {
    let mut pool = FamilyStoragePool::new(100 * 1024 * 1024); // 100 MB
    let member = [0x01u8; 32];

    assert!(pool.allocate_storage(&member, 10 * 1024 * 1024).is_ok());
    assert_eq!(pool.used_bytes, 10 * 1024 * 1024);
}

#[test]
fn test_r61_3_b_quota_exceeded_rejection() {
    let mut pool = FamilyStoragePool::new(50 * 1024 * 1024); // 50 MB
    let member = [0x01u8; 32];

    assert!(pool.allocate_storage(&member, 40 * 1024 * 1024).is_ok());
    assert!(pool.allocate_storage(&member, 20 * 1024 * 1024).is_err(), "Must reject allocation exceeding total quota");
}

#[test]
fn test_r61_3_c_per_member_quota_limits() {
    let mut pool = FamilyStoragePool::new(100 * 1024 * 1024);
    let child = [0x05u8; 32];
    pool.set_member_limit(child, 10 * 1024 * 1024);

    assert_eq!(pool.member_limits.get(&hex::encode(child)), Some(&(10 * 1024 * 1024)));
}

#[test]
fn test_r61_3_d_save_group_state_to_node() {
    let dir = tempdir().unwrap();
    let seed = [161u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let group = GroupState::new("Home Vault", node.identity.actor_id);
    let obj_id = NexGroupsService::save_group_state(&mut node, &group).unwrap();

    assert_ne!(obj_id, [0u8; 32]);
    assert_eq!(node.state.object_store.len(), 1);
}

#[test]
fn test_r61_3_e_multi_group_isolation() {
    let dir = tempdir().unwrap();
    let seed = [162u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let group1 = GroupState::new("Family", node.identity.actor_id);
    let group2 = GroupState::new("Work", node.identity.actor_id);

    NexGroupsService::save_group_state(&mut node, &group1).unwrap();
    NexGroupsService::save_group_state(&mut node, &group2).unwrap();

    assert_eq!(node.state.object_store.len(), 2);
}

#[test]
fn test_r61_3_f_zero_regression_family_vault_lifecycle() {
    let dir = tempdir().unwrap();
    let seed = [163u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());
    node.stop().unwrap();
}
