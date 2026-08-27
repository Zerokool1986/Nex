use std::collections::BTreeMap;
use crate::runtime::node::NexNode;
use crate::runtime::shell::{NexHomeShell, SpaceType, HomeFeedItem};
use crate::runtime::panels::{ContextualPanelsEngine, StoragePanelModel};
use crate::runtime::diagnostics::{SubstrateHealthDiagnostics, ProgressiveTier};
use crate::runtime::dispatcher::UiActionDispatcher;
use crate::object::types::{ObjectID, ObjectType, NamespaceID};
use crate::identity::types::{ActorID, CapabilityProof, DeviceCertificate, OP_READ, OP_WRITE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceComplexity {
    Simple,
    Standard,
    Advanced,
    Expert,
}

#[derive(Debug, Clone)]
pub struct HomeScreenViewModel {
    pub active_space: SpaceType,
    pub space_title: String,
    pub sync_status_label: String,
    pub storage_health_label: String,
    pub identity_protection_label: String,
    pub feed_items: Vec<FeedItemViewModel>,
    pub total_items_in_space: usize,
    pub available_spaces: Vec<SpaceType>,
}

#[derive(Debug, Clone)]
pub struct FeedItemViewModel {
    pub object_id_hex: String,
    pub title: String,
    pub subtitle: String,
    pub object_type: ObjectType,
    pub status_badge: String,
    pub timestamp_label: String,
    pub shared_badge: String,
}

#[derive(Debug, Clone)]
pub struct PhotosScreenViewModel {
    pub active_space: SpaceType,
    pub total_photos: usize,
    pub photo_cards: Vec<PhotoCardViewModel>,
    pub storage_used_label: String,
    pub sync_status_label: String,
}

#[derive(Debug, Clone)]
pub struct PhotoCardViewModel {
    pub object_id: ObjectID,
    pub object_id_hex: String,
    pub title: String,
    pub byte_size: usize,
    pub byte_size_formatted: String,
    pub space: String,
    pub status_badge: String,
    pub technical_details: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DriveScreenViewModel {
    pub active_space: SpaceType,
    pub total_files: usize,
    pub file_rows: Vec<FileRowViewModel>,
    pub storage_used_label: String,
}

#[derive(Debug, Clone)]
pub struct FileRowViewModel {
    pub object_id: ObjectID,
    pub filename: String,
    pub byte_size_formatted: String,
    pub status_badge: String,
    pub technical_details: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ObjectDetailViewModel {
    pub object_id_hex: String,
    pub title: String,
    pub object_type_label: String,
    pub space_label: String,
    pub byte_size: usize,
    pub status_badge: String,
    pub sharing_label: String,
    pub advanced_diagnostics: Option<String>,
}

pub struct HumanExperienceEngine;

impl HumanExperienceEngine {
    fn complexity_to_progressive_tier(complexity: InterfaceComplexity) -> ProgressiveTier {
        match complexity {
            InterfaceComplexity::Simple => ProgressiveTier::Everyday,
            InterfaceComplexity::Standard => ProgressiveTier::Informational,
            InterfaceComplexity::Advanced | InterfaceComplexity::Expert => ProgressiveTier::Advanced,
        }
    }

    pub fn render_home_screen(
        node: &NexNode,
        active_space: SpaceType,
        complexity: InterfaceComplexity,
    ) -> HomeScreenViewModel {
        let mut shell = NexHomeShell::new();
        shell.switch_space(active_space);

        let summary = shell.generate_home_summary(node);
        let tier = Self::complexity_to_progressive_tier(complexity);

        let sync_label = SubstrateHealthDiagnostics::format_sync_state(node, tier);
        let storage_label = SubstrateHealthDiagnostics::format_storage_state(node, tier);
        let root_actor = node.identity.actor_id;
        let identity_label = SubstrateHealthDiagnostics::format_identity_state(&root_actor, 1, tier);

        let space_ns = NexHomeShell::space_to_namespace(active_space);
        let feed_items: Vec<FeedItemViewModel> = node.state.object_store.values()
            .filter(|o| o.namespace == space_ns && !o.tombstoned)
            .map(|o| {
                let title = o.metadata.get("title")
                    .or_else(|| o.metadata.get("filename"))
                    .cloned()
                    .unwrap_or_else(|| "Untitled Object".to_string());

                let status = match complexity {
                    InterfaceComplexity::Simple => "Protected".to_string(),
                    InterfaceComplexity::Standard => "Synced (Local mesh)".to_string(),
                    InterfaceComplexity::Advanced => format!("CAS: {}B | Lamport: {}", o.payload_bytes.len(), o.created_lamport),
                    InterfaceComplexity::Expert => format!("SMT Node | Owner: {}", hex::encode(&o.owner_actor_id[0..4])),
                };

                FeedItemViewModel {
                    object_id_hex: hex::encode(o.object_id),
                    title,
                    subtitle: format!("{:?}", o.object_type),
                    object_type: o.object_type,
                    status_badge: status,
                    timestamp_label: format!("Epoch {}", o.created_epoch),
                    shared_badge: format!("Shared: {:?}", active_space),
                }
            })
            .collect();

        HomeScreenViewModel {
            active_space,
            space_title: format!("{:?} Space", active_space),
            sync_status_label: sync_label,
            storage_health_label: storage_label,
            identity_protection_label: identity_label,
            feed_items,
            total_items_in_space: summary.total_objects_in_space,
            available_spaces: vec![
                SpaceType::Personal,
                SpaceType::Family,
                SpaceType::Work,
                SpaceType::Community,
                SpaceType::Project,
            ],
        }
    }

    pub fn render_photos_screen(
        node: &NexNode,
        active_space: SpaceType,
        complexity: InterfaceComplexity,
    ) -> PhotosScreenViewModel {
        let space_ns = NexHomeShell::space_to_namespace(active_space);
        let photos: Vec<&crate::object::types::NexObject> = node.state.object_store.values()
            .filter(|o| o.namespace == space_ns && o.object_type == ObjectType::PhotoMedia && !o.tombstoned)
            .collect();

        let total_bytes: usize = photos.iter().map(|o| o.payload_bytes.len()).sum();
        let cards: Vec<PhotoCardViewModel> = photos.into_iter().map(|o| {
            let title = o.metadata.get("title").cloned().unwrap_or_else(|| "Untitled Photo".to_string());
            let status = match complexity {
                InterfaceComplexity::Simple => "Protected".to_string(),
                InterfaceComplexity::Standard => "Available offline & Synced".to_string(),
                InterfaceComplexity::Advanced | InterfaceComplexity::Expert => {
                    format!("CAS Inode verified | Lamport {}", o.created_lamport)
                }
            };

            let technical = if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                Some(format!("Owner: {} | NS: {}", hex::encode(&o.owner_actor_id[0..4]), hex::encode(&o.namespace[0..4])))
            } else {
                None
            };

            PhotoCardViewModel {
                object_id: o.object_id,
                object_id_hex: hex::encode(o.object_id),
                title,
                byte_size: o.payload_bytes.len(),
                byte_size_formatted: format!("{:.1} KB", o.payload_bytes.len() as f64 / 1024.0),
                space: format!("{:?}", active_space),
                status_badge: status,
                technical_details: technical,
            }
        }).collect();

        PhotosScreenViewModel {
            active_space,
            total_photos: cards.len(),
            photo_cards: cards,
            storage_used_label: format!("{:.2} KB used", total_bytes as f64 / 1024.0),
            sync_status_label: "Up to date".to_string(),
        }
    }

    pub fn render_drive_screen(
        node: &NexNode,
        active_space: SpaceType,
        complexity: InterfaceComplexity,
    ) -> DriveScreenViewModel {
        let space_ns = NexHomeShell::space_to_namespace(active_space);
        let docs: Vec<&crate::object::types::NexObject> = node.state.object_store.values()
            .filter(|o| o.namespace == space_ns && o.object_type == ObjectType::DriveInode && !o.tombstoned)
            .collect();

        let total_bytes: usize = docs.iter().map(|o| o.payload_bytes.len()).sum();
        let rows: Vec<FileRowViewModel> = docs.into_iter().map(|o| {
            let filename = o.metadata.get("filename").cloned().unwrap_or_else(|| "document.bin".to_string());
            let status = match complexity {
                InterfaceComplexity::Simple => "Protected".to_string(),
                InterfaceComplexity::Standard => "Synced".to_string(),
                InterfaceComplexity::Advanced | InterfaceComplexity::Expert => {
                    format!("WAL Inode | Epoch {}", o.created_epoch)
                }
            };

            let technical = if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                Some(format!("Object Hash: {}", hex::encode(o.object_id)))
            } else {
                None
            };

            FileRowViewModel {
                object_id: o.object_id,
                filename,
                byte_size_formatted: format!("{:.1} KB", o.payload_bytes.len() as f64 / 1024.0),
                status_badge: status,
                technical_details: technical,
            }
        }).collect();

        DriveScreenViewModel {
            active_space,
            total_files: rows.len(),
            file_rows: rows,
            storage_used_label: format!("{:.2} KB used", total_bytes as f64 / 1024.0),
        }
    }

    pub fn render_object_detail(
        node: &NexNode,
        object_id: &ObjectID,
        complexity: InterfaceComplexity,
    ) -> Result<ObjectDetailViewModel, String> {
        let obj = node.state.object_store.get(object_id)
            .ok_or_else(|| format!("Object {} not found", hex::encode(object_id)))?;

        let title = obj.metadata.get("title")
            .or_else(|| obj.metadata.get("filename"))
            .cloned()
            .unwrap_or_else(|| "Untitled".to_string());

        let space_str = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());

        let status = match complexity {
            InterfaceComplexity::Simple => "Protected on all your devices".to_string(),
            InterfaceComplexity::Standard => "Verified & Synchronized".to_string(),
            InterfaceComplexity::Advanced => format!("State Root Match | Schema v{}", obj.schema_version),
            InterfaceComplexity::Expert => format!("SMT Key: {} | Author: {}", hex::encode(obj.object_id), hex::encode(obj.owner_actor_id)),
        };

        let advanced = if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
            Some(format!("Raw Inode: SchemaVersion={}, CreatedEpoch={}, CreatedLamport={}",
                obj.schema_version, obj.created_epoch, obj.created_lamport))
        } else {
            None
        };

        Ok(ObjectDetailViewModel {
            object_id_hex: hex::encode(obj.object_id),
            title,
            object_type_label: format!("{:?}", obj.object_type),
            space_label: space_str,
            byte_size: obj.payload_bytes.len(),
            status_badge: status,
            sharing_label: "Explicit Capability Proof Required".to_string(),
            advanced_diagnostics: advanced,
        })
    }
}
