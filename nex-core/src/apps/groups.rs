use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::runtime::node::NexNode;
use crate::object::types::{NamespaceID, ObjectType};
use crate::api::NexAppApi;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupRole {
    Admin,
    Member,
    Guest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMember {
    pub actor_id: [u8; 32],
    pub role: GroupRole,
    pub joined_epoch: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupState {
    pub group_id: [u8; 32],
    pub name: String,
    pub epoch: u64,
    pub members: BTreeMap<String, GroupMember>,
    pub epoch_secret: [u8; 32],
}

impl GroupState {
    pub fn new(name: &str, creator_actor: [u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/GROUP_ID/v1");
        hasher.update(name.as_bytes());
        hasher.update(&creator_actor);
        let group_id: [u8; 32] = hasher.finalize().into();

        let mut init_hasher = Sha256::new();
        init_hasher.update(b"NEX/GROUP_INITIAL_SECRET/v1");
        init_hasher.update(&group_id);
        let epoch_secret: [u8; 32] = init_hasher.finalize().into();

        let mut members = BTreeMap::new();
        let key = hex::encode(creator_actor);
        members.insert(key, GroupMember {
            actor_id: creator_actor,
            role: GroupRole::Admin,
            joined_epoch: 1,
            is_active: true,
        });

        Self {
            group_id,
            name: name.to_string(),
            epoch: 1,
            members,
            epoch_secret,
        }
    }

    pub fn add_member(&mut self, actor_id: [u8; 32], role: GroupRole) {
        let key = hex::encode(actor_id);
        self.members.insert(key, GroupMember {
            actor_id,
            role,
            joined_epoch: self.epoch,
            is_active: true,
        });
    }

    pub fn remove_member(&mut self, actor_id: &[u8; 32]) -> Result<(), String> {
        let key = hex::encode(actor_id);
        if let Some(m) = self.members.get_mut(&key) {
            m.is_active = false;
            // Advance epoch and ratchet key for forward secrecy
            self.epoch += 1;
            let mut hasher = Sha256::new();
            hasher.update(b"NEX/GROUP_RATCHET/v1");
            hasher.update(&self.epoch_secret);
            hasher.update(&self.epoch.to_be_bytes());
            self.epoch_secret = hasher.finalize().into();
            Ok(())
        } else {
            Err("Member not found".to_string())
        }
    }

    pub fn is_active_member(&self, actor_id: &[u8; 32]) -> bool {
        let key = hex::encode(actor_id);
        self.members.get(&key).map_or(false, |m| m.is_active)
    }

    pub fn is_admin(&self, actor_id: &[u8; 32]) -> bool {
        let key = hex::encode(actor_id);
        self.members.get(&key).map_or(false, |m| m.is_active && m.role == GroupRole::Admin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyStoragePool {
    pub total_quota_bytes: u64,
    pub used_bytes: u64,
    pub member_limits: BTreeMap<String, u64>,
}

impl FamilyStoragePool {
    pub fn new(total_quota_bytes: u64) -> Self {
        Self {
            total_quota_bytes,
            used_bytes: 0,
            member_limits: BTreeMap::new(),
        }
    }

    pub fn set_member_limit(&mut self, actor: [u8; 32], limit: u64) {
        self.member_limits.insert(hex::encode(actor), limit);
    }

    pub fn allocate_storage(&mut self, _actor: &[u8; 32], bytes: u64) -> Result<(), String> {
        if self.used_bytes + bytes > self.total_quota_bytes {
            return Err("Storage pool quota exceeded".to_string());
        }
        self.used_bytes += bytes;
        Ok(())
    }
}

pub struct NexGroupsService;

impl NexGroupsService {
    pub const GROUPS_NAMESPACE: NamespaceID = [0xBB; 32];

    pub fn save_group_state(node: &mut NexNode, group: &GroupState) -> Result<[u8; 32], String> {
        let mut meta = BTreeMap::new();
        meta.insert("group_id".to_string(), hex::encode(group.group_id));
        meta.insert("name".to_string(), group.name.clone());
        meta.insert("epoch".to_string(), group.epoch.to_string());
        meta.insert("members_count".to_string(), group.members.len().to_string());

        let payload = serde_json::to_vec(group).map_err(|e| format!("{:?}", e))?;
        node.create_object(Self::GROUPS_NAMESPACE, ObjectType::Synthetic(15), meta, payload)
            .map_err(|e| format!("{:?}", e))
    }
}
