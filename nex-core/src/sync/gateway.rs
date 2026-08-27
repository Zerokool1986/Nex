use std::collections::{BTreeMap, BTreeSet};
use crate::identity::types::{ActorID, CapabilityProof, DeviceCertificate, OP_WRITE};
use crate::identity::verifier::{verify_capability_chain, verify_device_certificate_with_crl};
use crate::object::types::{NamespaceID, ObjectID};

pub struct SyncCapabilityGateway;

impl SyncCapabilityGateway {
    pub fn verify_sync_ingest(
        cert: Option<&DeviceCertificate>,
        proof: &CapabilityProof,
        namespace: &NamespaceID,
        object_id: &ObjectID,
        current_epoch: u64,
        revoked_devices: &BTreeSet<ActorID>,
        revoked_tokens: &BTreeMap<[u8; 32], u64>,
        root_actor: &ActorID,
    ) -> Result<ActorID, String> {
        // 1. If device certificate is presented, verify it against CRL
        if let Some(device_cert) = cert {
            verify_device_certificate_with_crl(device_cert, root_actor, current_epoch, revoked_devices)
                .map_err(|e| format!("DeviceAuthFailed: {:?}", e))?;
        }

        // 2. Verify capability token chain for OP_WRITE
        verify_capability_chain(
            proof,
            OP_WRITE,
            namespace,
            Some(object_id),
            current_epoch,
            revoked_tokens,
            root_actor,
        ).map_err(|e| format!("CapabilityAuthFailed: {:?}", e))
    }
}
