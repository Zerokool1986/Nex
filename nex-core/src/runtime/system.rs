use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use crate::identity::types::{ActorID, DeviceCertificate, KeyType};
use crate::identity::verifier::derive_actor_id;

pub const DOMAIN_MNEMONIC_SALT: &[u8] = b"NEX/MNEMONIC/SALT/v1";
pub const DOMAIN_PAIRING_QR: &[u8] = b"NEX/PAIRING_QR/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SovereignMnemonic {
    pub words: Vec<String>,
}

impl SovereignMnemonic {
    pub fn generate_24_words(entropy_seed: &[u8; 32]) -> Self {
        let wordlist = [
            "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
            "absurd", "abuse", "access", "accident", "account", "accuse", "achieve", "acid",
            "acoustic", "acquire", "across", "act", "action", "actor", "actress", "actual",
            "adapt", "add", "addict", "address", "adjust", "admit", "adult", "advance"
        ];
        let mut words = Vec::with_capacity(24);
        for i in 0..24 {
            let idx = (entropy_seed[i] as usize) % wordlist.len();
            words.push(wordlist[idx].to_string());
        }
        Self { words }
    }

    pub fn to_signing_key(&self) -> SigningKey {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_MNEMONIC_SALT);
        for word in &self.words {
            hasher.update(word.as_bytes());
            hasher.update(b" ");
        }
        let seed: [u8; 32] = hasher.finalize().into();
        SigningKey::from_bytes(&seed)
    }

    pub fn to_actor_id(&self) -> ActorID {
        let key = self.to_signing_key();
        derive_actor_id(KeyType::Ed25519, key.verifying_key().as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingPayload {
    pub master_actor_id: ActorID,
    pub epoch: u64,
    pub nonce: u64,
    pub master_pubkey: Vec<u8>,
    pub signature: Vec<u8>,
}

pub struct PairingSession;

impl PairingSession {
    pub fn generate_pairing_payload(master_key: &SigningKey, epoch: u64, nonce: u64) -> PairingPayload {
        let pubkey = master_key.verifying_key().to_bytes().to_vec();
        let master_actor_id = derive_actor_id(KeyType::Ed25519, &pubkey);

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_PAIRING_QR);
        hasher.update(&master_actor_id);
        hasher.update(&epoch.to_le_bytes());
        hasher.update(&nonce.to_le_bytes());
        let digest = hasher.finalize();

        let sig = master_key.sign(&digest).to_bytes().to_vec();

        PairingPayload {
            master_actor_id,
            epoch,
            nonce,
            master_pubkey: pubkey,
            signature: sig,
        }
    }

    pub fn verify_and_enroll(
        payload: &PairingPayload,
        current_epoch: u64,
        device_key: &SigningKey,
    ) -> Result<DeviceCertificate, String> {
        if payload.epoch < current_epoch {
            return Err("PairingExpired: QR payload epoch is older than current network epoch".into());
        }

        // 1. Verify Master Signature
        if payload.master_pubkey.len() != 32 {
            return Err("Invalid master pubkey length".into());
        }
        let mut pk_bytes = [0u8; 32];
        pk_bytes.copy_from_slice(&payload.master_pubkey);
        let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|e| format!("Invalid master verifying key: {:?}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_PAIRING_QR);
        hasher.update(&payload.master_actor_id);
        hasher.update(&payload.epoch.to_le_bytes());
        hasher.update(&payload.nonce.to_le_bytes());
        let digest = hasher.finalize();

        if payload.signature.len() != 64 {
            return Err("Invalid master signature length".into());
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&payload.signature);
        let signature = Signature::from_bytes(&sig_bytes);

        verifying_key.verify(&digest, &signature)
            .map_err(|_| "InvalidMasterSignature: QR code signature verification failed".to_string())?;

        // 2. Issue Device Certificate
        let dev_pubkey = device_key.verifying_key().to_bytes().to_vec();
        let device_actor_id = derive_actor_id(KeyType::Ed25519, &dev_pubkey);

        let mut cert_hasher = Sha256::new();
        cert_hasher.update(b"NEX/DEVICE_CERT/v1");
        cert_hasher.update(&payload.master_actor_id);
        cert_hasher.update(&device_actor_id);
        cert_hasher.update(&payload.epoch.to_le_bytes());
        cert_hasher.update(&(payload.epoch + 1000).to_le_bytes());
        let cert_digest = cert_hasher.finalize();

        // Enrolled cert signature
        let dev_sig = device_key.sign(&cert_digest).to_bytes().to_vec();

        Ok(DeviceCertificate {
            master_actor_id: payload.master_actor_id,
            device_actor_id,
            not_before_epoch: payload.epoch,
            expires_at_epoch: payload.epoch + 1000,
            master_pubkey: Some(payload.master_pubkey.clone()),
            signature: dev_sig,
        })
    }
}
