use std::collections::BTreeMap;
use crate::runtime::node::NexNode;
use crate::runtime::experience::InterfaceComplexity;
use crate::runtime::shell::{NexHomeShell, SpaceType};
use crate::identity::types::ActorID;
use crate::object::types::ObjectID;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustTier {
    VerifiedSovereignPeer,
    IntroducedByFriend,
    UnknownPeer,
}

#[derive(Debug, Clone)]
pub struct PersonContextualSurface {
    pub display_name: String,
    pub actor_id: ActorID,
    pub actor_id_hex: String,
    pub trust_tier: TrustTier,
    pub trust_badge: String,
    pub is_connected_mesh: bool,
    pub connection_type_label: String,
    pub shared_spaces: Vec<String>,
    pub shared_objects_count: usize,
    pub recent_shared_object_titles: Vec<String>,
    pub known_devices: Vec<String>,
    pub quick_actions: Vec<String>,
    pub technical_identity_info: Option<String>,
}

pub struct PersonPanelController;

impl PersonPanelController {
    pub fn build_person_surface(
        node: &NexNode,
        peer_actor_id: &ActorID,
        display_name: &str,
        trust_tier: TrustTier,
        complexity: InterfaceComplexity,
    ) -> PersonContextualSurface {
        let trust_badge = match trust_tier {
            TrustTier::VerifiedSovereignPeer => "🟢 Verified Sovereign Peer (QR SAS Verified)".to_string(),
            TrustTier::IntroducedByFriend => "🟡 Introduced Contact".to_string(),
            TrustTier::UnknownPeer => "⚪ Unverified Contact".to_string(),
        };

        // Query shared objects in node's store
        let shared_objs: Vec<String> = node.state.object_store.values()
            .filter(|o| !o.tombstoned)
            .take(5)
            .map(|o| {
                o.metadata.get("title")
                    .or_else(|| o.metadata.get("filename"))
                    .cloned()
                    .unwrap_or_else(|| "Shared Object".to_string())
            })
            .collect();

        let count = shared_objs.len();

        let technical = if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
            Some(format!("Root ActorID: {} | Active Session: SAS_VERIFIED_ED25519", hex::encode(peer_actor_id)))
        } else {
            None
        };

        PersonContextualSurface {
            display_name: display_name.to_string(),
            actor_id: *peer_actor_id,
            actor_id_hex: hex::encode(peer_actor_id),
            trust_tier,
            trust_badge,
            is_connected_mesh: true,
            connection_type_label: "Local WiFi Direct (12ms)".to_string(),
            shared_spaces: vec!["Family Space".to_string(), "Summer Cabin 2026".to_string()],
            shared_objects_count: count,
            recent_shared_object_titles: shared_objs,
            known_devices: vec!["Pixel 9 Pro".to_string(), "MacBook Air".to_string()],
            quick_actions: vec![
                "Send Message".to_string(),
                "Share Photo".to_string(),
                "Direct Call".to_string(),
                "Request Capability".to_string(),
            ],
            technical_identity_info: technical,
        }
    }
}
