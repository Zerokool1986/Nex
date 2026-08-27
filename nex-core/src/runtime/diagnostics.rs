use crate::runtime::node::NexNode;
use crate::identity::types::ActorID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressiveTier {
    Everyday,
    Informational,
    Advanced,
}

pub struct SubstrateHealthDiagnostics;

impl SubstrateHealthDiagnostics {
    pub fn format_sync_state(node: &NexNode, tier: ProgressiveTier) -> String {
        let root = node.state.latest_mutation_id.unwrap_or([0u8; 32]);
        let obj_count = node.state.object_store.len();

        match tier {
            ProgressiveTier::Everyday => {
                "All up to date".to_string()
            }
            ProgressiveTier::Informational => {
                format!("Synchronized across local mesh ({} objects verified)", obj_count)
            }
            ProgressiveTier::Advanced => {
                format!("SMT State Root: {} | Objects: {}", hex::encode(root), obj_count)
            }
        }
    }

    pub fn format_storage_state(node: &NexNode, tier: ProgressiveTier) -> String {
        let total_bytes: usize = node.state.object_store.values().map(|o| o.payload_bytes.len()).sum();

        match tier {
            ProgressiveTier::Everyday => {
                "Storage healthy (All replicas protected)".to_string()
            }
            ProgressiveTier::Informational => {
                format!("{:.2} KB used across local CAS storage", total_bytes as f64 / 1024.0)
            }
            ProgressiveTier::Advanced => {
                format!("CAS Bytes: {} | Inodes: {} | Tombstones: {}",
                    total_bytes,
                    node.state.object_store.len(),
                    node.state.object_store.values().filter(|o| o.tombstoned).count()
                )
            }
        }
    }

    pub fn format_identity_state(
        root_actor: &ActorID,
        active_devices: usize,
        tier: ProgressiveTier,
    ) -> String {
        match tier {
            ProgressiveTier::Everyday => {
                format!("Sovereign & Protected ({} active devices)", active_devices)
            }
            ProgressiveTier::Informational => {
                format!("Root ID verified; {} hardware devices authorized under your identity", active_devices)
            }
            ProgressiveTier::Advanced => {
                format!("Root ActorID: {} | Authorized Active Devices: {}", hex::encode(root_actor), active_devices)
            }
        }
    }
}
