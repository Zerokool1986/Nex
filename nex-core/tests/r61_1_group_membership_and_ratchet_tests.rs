use nex_core::apps::groups::*;

#[test]
fn test_r61_1_a_group_creation_and_admin() {
    let admin = [0x01u8; 32];
    let group = GroupState::new("Sovereign Family", admin);

    assert_eq!(group.name, "Sovereign Family");
    assert_eq!(group.epoch, 1);
    assert!(group.is_admin(&admin));
    assert!(group.is_active_member(&admin));
}

#[test]
fn test_r61_1_b_add_members_roles() {
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Family", admin);

    let member = [0x02u8; 32];
    let guest = [0x03u8; 32];

    group.add_member(member, GroupRole::Member);
    group.add_member(guest, GroupRole::Guest);

    assert!(group.is_active_member(&member));
    assert!(!group.is_admin(&member));
    assert!(group.is_active_member(&guest));
    assert!(!group.is_admin(&guest));
    assert_eq!(group.members.len(), 3);
}

#[test]
fn test_r61_1_c_member_revocation_and_key_ratchet() {
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Team", admin);

    let member = [0x02u8; 32];
    group.add_member(member, GroupRole::Member);
    let secret_epoch_1 = group.epoch_secret;

    // Remove member
    assert!(group.remove_member(&member).is_ok());

    assert!(!group.is_active_member(&member));
    assert_eq!(group.epoch, 2);
    assert_ne!(group.epoch_secret, secret_epoch_1, "Group epoch secret must ratchet upon member revocation");
}

#[test]
fn test_r61_1_d_non_member_checks() {
    let admin = [0x01u8; 32];
    let group = GroupState::new("Private", admin);

    let stranger = [0x99u8; 32];
    assert!(!group.is_active_member(&stranger));
    assert!(!group.is_admin(&stranger));
}

#[test]
fn test_r61_1_e_multiple_ratchet_epochs() {
    let admin = [0x01u8; 32];
    let mut group = GroupState::new("Dynamic Group", admin);

    let mut secrets = Vec::new();
    secrets.push(group.epoch_secret);

    for i in 1..=5 {
        let m = [i as u8; 32];
        group.add_member(m, GroupRole::Member);
        group.remove_member(&m).unwrap();
        assert!(!secrets.contains(&group.epoch_secret));
        secrets.push(group.epoch_secret);
    }
    assert_eq!(group.epoch, 6);
}

#[test]
fn test_r61_1_f_zero_regression_group_lifecycle() {
    let admin = [0x11u8; 32];
    let mut group = GroupState::new("Test", admin);
    for i in 0..10 {
        let m = [i + 20; 32];
        group.add_member(m, GroupRole::Member);
        assert!(group.is_active_member(&m));
    }
}
