use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::identity::types::ActorID;

pub type ObjectID = [u8; 32];
pub type NamespaceID = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum ObjectType {
    DriveInode   = 0x0101,
    DriveFolder  = 0x0102,
    PhotoMedia   = 0x0201,
    PhotoAlbum   = 0x0202,
    ChatChannel  = 0x0301,
    ChatMessage  = 0x0302,
    ChatReceipt  = 0x0303,
    Community    = 0x0401,
    MemberRole   = 0x0402,
    VaultItem    = 0x0501,
    BackupIndex  = 0x0601,
    Synthetic(u16),
}

impl ObjectType {
    pub fn as_u16(&self) -> u16 {
        match self {
            ObjectType::DriveInode => 0x0101,
            ObjectType::DriveFolder => 0x0102,
            ObjectType::PhotoMedia => 0x0201,
            ObjectType::PhotoAlbum => 0x0202,
            ObjectType::ChatChannel => 0x0301,
            ObjectType::ChatMessage => 0x0302,
            ObjectType::ChatReceipt => 0x0303,
            ObjectType::Community => 0x0401,
            ObjectType::MemberRole => 0x0402,
            ObjectType::VaultItem => 0x0501,
            ObjectType::BackupIndex => 0x0601,
            ObjectType::Synthetic(v) => *v,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NexObject {
    pub object_id: ObjectID,
    pub object_type: ObjectType,
    pub namespace: NamespaceID,
    pub owner_actor_id: ActorID,
    pub schema_version: u16,
    pub created_epoch: u64,
    pub created_lamport: u64,
    #[serde(default)]
    pub winning_mutation_id: [u8; 32],
    pub metadata: BTreeMap<String, String>,
    pub payload_bytes: Vec<u8>,
    pub tombstoned: bool,
}
