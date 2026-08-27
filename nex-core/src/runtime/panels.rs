use serde::{Deserialize, Serialize};
use crate::runtime::node::NexNode;
use crate::identity::types::{ActorID, DeviceCertificate};
use crate::object::types::ObjectType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonPanelModel {
    pub actor_id: ActorID,
    pub display_name: String,
    pub trust_tier: String,
    pub shared_objects_count: usize,
    pub direct_chat_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevicePanelModel {
    pub device_actor_id: ActorID,
    pub is_local_device: bool,
    pub not_before_epoch: u64,
    pub expires_at_epoch: u64,
    pub is_revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePanelModel {
    pub total_used_bytes: usize,
    pub photos_bytes: usize,
    pub drive_bytes: usize,
    pub vault_bytes: usize,
    pub other_bytes: usize,
    pub objects_count: usize,
}

pub struct ContextualPanelsEngine;

impl ContextualPanelsEngine {
    pub fn project_person_panel(
        node: &NexNode,
        target_actor: &ActorID,
        display_name: &str,
    ) -> PersonPanelModel {
        let shared_count = node.state.object_store
            .values()
            .filter(|o| o.owner_actor_id == *target_actor && !o.tombstoned)
            .count();

        PersonPanelModel {
            actor_id: *target_actor,
            display_name: display_name.to_string(),
            trust_tier: "Verified Personally".to_string(),
            shared_objects_count: shared_count,
            direct_chat_available: true,
        }
    }

    pub fn project_device_panel(
        node: &NexNode,
        device_actor: &ActorID,
        cert: Option<&DeviceCertificate>,
        is_revoked: bool,
    ) -> DevicePanelModel {
        let is_local = *device_actor == node.identity.actor_id;
        let (nb, exp) = if let Some(c) = cert {
            (c.not_before_epoch, c.expires_at_epoch)
        } else {
            (0, u64::MAX)
        };

        DevicePanelModel {
            device_actor_id: *device_actor,
            is_local_device: is_local,
            not_before_epoch: nb,
            expires_at_epoch: exp,
            is_revoked,
        }
    }

    pub fn project_storage_panel(node: &NexNode) -> StoragePanelModel {
        let mut total = 0;
        let mut photos = 0;
        let mut drive = 0;
        let mut vault = 0;
        let mut other = 0;
        let mut count = 0;

        for obj in node.state.object_store.values() {
            if obj.tombstoned { continue; }
            let len = obj.payload_bytes.len();
            total += len;
            count += 1;

            match obj.object_type {
                ObjectType::PhotoMedia | ObjectType::PhotoAlbum => photos += len,
                ObjectType::DriveInode | ObjectType::DriveFolder => drive += len,
                ObjectType::VaultItem => vault += len,
                _ => other += len,
            }
        }

        StoragePanelModel {
            total_used_bytes: total,
            photos_bytes: photos,
            drive_bytes: drive,
            vault_bytes: vault,
            other_bytes: other,
            objects_count: count,
        }
    }
}
