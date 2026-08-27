use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use crate::identity::types::ActorID;

pub const DOMAIN_DISCOVERY_ADV: &[u8] = b"NEX/DISCOVERY_ADV/v1";
pub const DOMAIN_BLIND_TOPIC: &[u8] = b"NEX/BLIND_TOPIC/v1";

pub const TRANSPORT_TAG_LOCAL_IPC: u16 = 0x00;
pub const TRANSPORT_TAG_RETICULUM: u16 = 0x01;
pub const TRANSPORT_TAG_QUIC: u16 = 0x02;
pub const TRANSPORT_TAG_WEBRTC: u16 = 0x03;
pub const TRANSPORT_TAG_TCP: u16 = 0x04;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointHint {
    pub transport_tag: u16,
    pub address_bytes: Vec<u8>,
    pub priority: u8, // Higher is preferred
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryAdvertisement {
    pub actor_id: ActorID,
    pub namespace_or_topic: [u8; 32],
    pub sequence: u64,
    pub not_before_epoch: u64,
    pub expires_at_epoch: u64,
    pub endpoint_hints: Vec<EndpointHint>,
    pub offered_capabilities: u32,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEntry {
    pub destination: ActorID,
    pub next_hop: ActorID,
    pub hop_count: u8,
    pub sequence: u64,
    pub expires_at_epoch: u64,
    pub endpoint_hints: Vec<EndpointHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    SignatureInvalid,
    ExpiredAdvertisement { current_epoch: u64, expires_at: u64 },
    PrematureAdvertisement { current_epoch: u64, not_before: u64 },
    StaleSequence { current_sequence: u64, incoming_sequence: u64 },
    RoutingTableFull,
    InvalidEndpoint,
}

pub fn derive_blinded_topic(namespace_secret: &[u8; 32], epoch: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_BLIND_TOPIC);
    hasher.update(namespace_secret);
    hasher.update(&epoch.to_le_bytes());
    hasher.finalize().into()
}

pub fn hash_discovery_advertisement_body(adv: &DiscoveryAdvertisement) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DISCOVERY_ADV);
    hasher.update(&adv.actor_id);
    hasher.update(&adv.namespace_or_topic);
    hasher.update(&adv.sequence.to_le_bytes());
    hasher.update(&adv.not_before_epoch.to_le_bytes());
    hasher.update(&adv.expires_at_epoch.to_le_bytes());
    for hint in &adv.endpoint_hints {
        hasher.update(&hint.transport_tag.to_le_bytes());
        hasher.update(&[hint.priority]);
        hasher.update(&hint.address_bytes);
    }
    hasher.update(&adv.offered_capabilities.to_le_bytes());
    hasher.finalize().into()
}
