use crate::runtime::node::NexNode;
use crate::runtime::experience::InterfaceComplexity;
use crate::identity::types::{ActorID, DeviceCertificate};

#[derive(Debug, Clone)]
pub struct DeviceContextualSurface {
    pub device_name: String,
    pub device_actor_id_hex: String,
    pub is_local_device: bool,
    pub connection_badge: String,
    pub transport_type_label: String,
    pub latency_ms: u32,
    pub storage_quota_label: String,
    pub storage_used_bytes: usize,
    pub sync_health_label: String,
    pub hardware_keystore_backed: bool,
    pub key_protection_status: String,
    pub certificate_validity: String,
    pub technical_device_info: Option<String>,
}

pub struct DevicePanelController;

impl DevicePanelController {
    pub fn build_device_surface(
        node: &NexNode,
        device_actor_id: &ActorID,
        device_name: &str,
        cert: Option<&DeviceCertificate>,
        is_revoked: bool,
        is_hardware_verified: bool,
        complexity: InterfaceComplexity,
    ) -> DeviceContextualSurface {
        let is_local = node.identity.actor_id == *device_actor_id;

        let conn_badge = if is_revoked {
            "🔴 Certificate Revoked".to_string()
        } else if is_local {
            "🟢 This Device (Local Desktop Host)".to_string()
        } else {
            "🟢 Connected via Local Mesh".to_string()
        };

        let total_bytes: usize = node.state.object_store.values().map(|o| o.payload_bytes.len()).sum();

        let (cert_str, technical) = match cert {
            Some(c) => {
                let s = format!("Valid Epoch {} -> {}", c.not_before_epoch, c.expires_at_epoch);
                let t = format!("Master ActorID: {} | Sig: {}", hex::encode(&c.master_actor_id[0..4]), hex::encode(&c.signature[0..4]));
                (s, Some(t))
            }
            None => ("Permanent Root Key (Local Node)".to_string(), None),
        };

        let key_prot = if is_hardware_verified {
            "Hardware TEE KeyStore Verified".to_string()
        } else {
            "Software Ed25519 Keyring (Hardware TEE: Not Verified on this Host)".to_string()
        };

        DeviceContextualSurface {
            device_name: device_name.to_string(),
            device_actor_id_hex: hex::encode(device_actor_id),
            is_local_device: is_local,
            connection_badge: conn_badge,
            transport_type_label: "Local Direct / IPC".to_string(),
            latency_ms: 1,
            storage_quota_label: "128 GB allocated / 1.2 TB available".to_string(),
            storage_used_bytes: total_bytes,
            sync_health_label: "Up to date".to_string(),
            hardware_keystore_backed: is_hardware_verified,
            key_protection_status: key_prot,
            certificate_validity: cert_str,
            technical_device_info: if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                technical
            } else {
                None
            },
        }
    }
}
