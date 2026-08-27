use std::collections::BTreeMap;
use crate::runtime::node::NexNode;
use crate::runtime::shell::{NexHomeShell, SpaceType};
use crate::runtime::experience::InterfaceComplexity;
use crate::runtime::diagnostics::ProgressiveTier;
use crate::sync::anti_entropy::{AntiEntropyEngine, SyncStreamBatch};
use crate::object::types::{ObjectID, ObjectType, NamespaceID};
use crate::identity::types::{ActorID, CapabilityProof};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkLinkState {
    ConnectedLocalWifi,
    ConnectedRelay,
    Reconnecting,
    Offline,
    PartialConnectivity,
    DegradedLossy(u8), // percentage packet loss: 0..100
}

#[derive(Debug, Clone)]
pub struct HumanNetworkStatusViewModel {
    pub link_state: NetworkLinkState,
    pub headline: String,
    pub detail_message: String,
    pub pending_items_count: usize,
    pub can_retry_now: bool,
    pub technical_error_code: Option<String>,
}

pub struct ProductionRealityEngine;

impl ProductionRealityEngine {
    pub fn format_network_status(
        state: NetworkLinkState,
        pending_items: usize,
        complexity: InterfaceComplexity,
    ) -> HumanNetworkStatusViewModel {
        match state {
            NetworkLinkState::ConnectedLocalWifi => {
                let (headline, detail) = if pending_items == 0 {
                    ("Up to date".to_string(), "Protected on your local mesh".to_string())
                } else {
                    ("Syncing…".to_string(), format!("Synchronizing {} items across devices", pending_items))
                };
                HumanNetworkStatusViewModel {
                    link_state: state,
                    headline,
                    detail_message: detail,
                    pending_items_count: pending_items,
                    can_retry_now: false,
                    technical_error_code: if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                        Some("LINK_OK_WIFI_DIRECT_P2P".to_string())
                    } else {
                        None
                    },
                }
            }
            NetworkLinkState::ConnectedRelay => {
                HumanNetworkStatusViewModel {
                    link_state: state,
                    headline: "Syncing via Relay".to_string(),
                    detail_message: "Encrypted store-and-forward relay active".to_string(),
                    pending_items_count: pending_items,
                    can_retry_now: false,
                    technical_error_code: if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                        Some("LINK_OK_OPAQUE_RELAY_V1".to_string())
                    } else {
                        None
                    },
                }
            }
            NetworkLinkState::Reconnecting => {
                HumanNetworkStatusViewModel {
                    link_state: state,
                    headline: "Reconnecting…".to_string(),
                    detail_message: "Attempting to re-establish secure mesh channel".to_string(),
                    pending_items_count: pending_items,
                    can_retry_now: true,
                    technical_error_code: if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                        Some("ERR_LINK_BACKOFF_EXPONENTIAL_RETRY".to_string())
                    } else {
                        None
                    },
                }
            }
            NetworkLinkState::Offline => {
                HumanNetworkStatusViewModel {
                    link_state: state,
                    headline: "You're offline".to_string(),
                    detail_message: "Changes will sync automatically when you're connected.".to_string(),
                    pending_items_count: pending_items,
                    can_retry_now: true,
                    technical_error_code: if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                        Some("ERR_NO_ROUTE_TO_HOST_PHYSICAL_DISCONNECT".to_string())
                    } else {
                        None
                    },
                }
            }
            NetworkLinkState::PartialConnectivity => {
                HumanNetworkStatusViewModel {
                    link_state: state,
                    headline: "Some items are waiting".to_string(),
                    detail_message: format!("{} items waiting for peer device to come online", pending_items),
                    pending_items_count: pending_items,
                    can_retry_now: true,
                    technical_error_code: if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                        Some("WARN_PARTIAL_MESH_TARGET_UNREACHABLE".to_string())
                    } else {
                        None
                    },
                }
            }
            NetworkLinkState::DegradedLossy(loss_pct) => {
                HumanNetworkStatusViewModel {
                    link_state: state,
                    headline: "Syncing slowly…".to_string(),
                    detail_message: "Weak connection detected. NEX is retransmitting packets safely.".to_string(),
                    pending_items_count: pending_items,
                    can_retry_now: false,
                    technical_error_code: if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                        Some(format!("WARN_PACKET_LOSS_RATE_{}_PCT_CRC_RETRY", loss_pct))
                    } else {
                        None
                    },
                }
            }
        }
    }

    /// Simulate lossy transport sync with packet drop rate (0..100)
    pub fn simulate_lossy_sync(
        mobile_node: &mut NexNode,
        desktop_node: &mut NexNode,
        drop_rate_pct: u8,
        seed: u64,
    ) -> (usize, usize) { // (successful_batches, dropped_batches)
        let session_id = [0x77; 16];
        let adv_desktop = AntiEntropyEngine::generate_advertise(desktop_node, session_id);
        let batches = AntiEntropyEngine::generate_batches_for_peer(
            mobile_node,
            session_id,
            &adv_desktop.frontier_mutation_ids,
            1, // single item batches for fine-grained drop simulation
        );

        let mut success = 0;
        let mut dropped = 0;

        for (i, batch) in batches.into_iter().enumerate() {
            // Deterministic pseudo-random drop check
            let pseudo_rand = ((seed.wrapping_add((i as u64).wrapping_mul(31))) % 100) as u8;
            if pseudo_rand < drop_rate_pct {
                dropped += 1;
                // Simulated dropped packet: not delivered to desktop
            } else {
                if AntiEntropyEngine::ingest_batch(desktop_node, batch).is_ok() {
                    success += 1;
                }
            }
        }

        // Reconcile metadata for objects that made it across
        for (id, obj) in &mobile_node.state.object_store {
            if let Some(target) = desktop_node.state.object_store.get_mut(id) {
                target.namespace = obj.namespace;
                target.object_type = obj.object_type;
                target.metadata = obj.metadata.clone();
                target.tombstoned = obj.tombstoned;
            }
        }

        (success, dropped)
    }
}
