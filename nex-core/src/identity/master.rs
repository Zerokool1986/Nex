use std::collections::BTreeSet;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Signature};
use sha2::{Sha256, Digest};
use crate::identity::types::{ActorID, KeyType, DeviceCertificate};
use crate::identity::verifier::{derive_actor_id, DOMAIN_DEVICE_CERT};

/// Master Sovereign Identity holder for cold storage and enrollment ceremonies.
pub struct NexMasterIdentity {
    master_signing_key: SigningKey,
    pub master_verifying_key: VerifyingKey,
    pub root_actor_id: ActorID,
}

impl NexMasterIdentity {
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        let root_actor_id = derive_actor_id(KeyType::Ed25519, &verifying_key.to_bytes());
        Self {
            master_signing_key: signing_key,
            master_verifying_key: verifying_key,
            root_actor_id,
        }
    }

    pub fn issue_device_certificate(
        &self,
        device_pubkey_bytes: &[u8; 32],
        not_before_epoch: u64,
        expires_at_epoch: u64,
    ) -> Result<DeviceCertificate, String> {
        let device_actor_id = derive_actor_id(KeyType::Ed25519, device_pubkey_bytes);

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_DEVICE_CERT);
        hasher.update(&self.root_actor_id);
        hasher.update(&device_actor_id);
        hasher.update(&not_before_epoch.to_le_bytes());
        hasher.update(&expires_at_epoch.to_le_bytes());
        let cert_hash: [u8; 32] = hasher.finalize().into();

        let sig: Signature = self.master_signing_key.sign(&cert_hash);

        Ok(DeviceCertificate {
            master_actor_id: self.root_actor_id,
            device_actor_id,
            not_before_epoch,
            expires_at_epoch,
            master_pubkey: Some(self.master_verifying_key.to_bytes().to_vec()),
            signature: sig.to_bytes().to_vec(),
        })
    }

    pub fn revoke_device(
        &self,
        crl: &mut BTreeSet<ActorID>,
        target_device_actor: ActorID,
    ) {
        crl.insert(target_device_actor);
    }
}
