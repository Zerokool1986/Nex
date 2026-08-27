use sha2::{Sha256, Digest};
use ed25519_dalek::SigningKey;
use crate::identity::master::NexMasterIdentity;
use crate::identity::types::DeviceCertificate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingPayload {
    pub session_id: [u8; 16],
    pub ephemeral_pubkey: [u8; 32],
    pub nonce: [u8; 16],
    pub expires_at_epoch: u64,
    pub rendezvous: String,
}

impl PairingPayload {
    pub fn encode_qr_uri(&self) -> String {
        format!(
            "nex://pair/v1?sid={}&pk={}&nonce={}&exp={}&rdv={}",
            hex::encode(self.session_id),
            hex::encode(self.ephemeral_pubkey),
            hex::encode(self.nonce),
            self.expires_at_epoch,
            self.rendezvous
        )
    }

    pub fn decode_qr_uri(uri: &str) -> Result<Self, String> {
        if !uri.starts_with("nex://pair/v1?") {
            return Err("InvalidUriScheme".into());
        }
        let query = &uri["nex://pair/v1?".len()..];
        let mut sid = None;
        let mut pk = None;
        let mut nonce = None;
        let mut exp = None;
        let mut rdv = None;

        for pair in query.split('&') {
            let mut kv = pair.split('=');
            let key = kv.next().ok_or("MalformedParam")?;
            let val = kv.next().ok_or("MalformedParam")?;
            match key {
                "sid" => {
                    let bytes = hex::decode(val).map_err(|e| e.to_string())?;
                    if bytes.len() != 16 { return Err("InvalidSidLen".into()); }
                    let mut arr = [0u8; 16];
                    arr.copy_from_slice(&bytes);
                    sid = Some(arr);
                }
                "pk" => {
                    let bytes = hex::decode(val).map_err(|e| e.to_string())?;
                    if bytes.len() != 32 { return Err("InvalidPkLen".into()); }
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    pk = Some(arr);
                }
                "nonce" => {
                    let bytes = hex::decode(val).map_err(|e| e.to_string())?;
                    if bytes.len() != 16 { return Err("InvalidNonceLen".into()); }
                    let mut arr = [0u8; 16];
                    arr.copy_from_slice(&bytes);
                    nonce = Some(arr);
                }
                "exp" => {
                    exp = Some(val.parse::<u64>().map_err(|e| e.to_string())?);
                }
                "rdv" => {
                    rdv = Some(val.to_string());
                }
                _ => {}
            }
        }

        Ok(Self {
            session_id: sid.ok_or("MissingSid")?,
            ephemeral_pubkey: pk.ok_or("MissingPk")?,
            nonce: nonce.ok_or("MissingNonce")?,
            expires_at_epoch: exp.ok_or("MissingExp")?,
            rendezvous: rdv.ok_or("MissingRdv")?,
        })
    }
}

pub fn compute_sas_code(
    initiator_pk: &[u8; 32],
    candidate_pk: &[u8; 32],
    nonce_a: &[u8; 16],
    nonce_b: &[u8; 16],
) -> u32 {
    let mut hasher = Sha256::new();
    hasher.update(b"NEX/PAIRING_SAS/v1");
    hasher.update(initiator_pk);
    hasher.update(candidate_pk);
    hasher.update(nonce_a);
    hasher.update(nonce_b);
    let hash: [u8; 32] = hasher.finalize().into();

    let num = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);
    num % 1_000_000 // 6-digit SAS
}

pub struct PairingSessionInitiator {
    pub session_id: [u8; 16],
    pub local_ephemeral_key: SigningKey,
    pub local_nonce: [u8; 16],
    pub expires_at_epoch: u64,
}

impl PairingSessionInitiator {
    pub fn new(session_id: [u8; 16], key_seed: &[u8; 32], local_nonce: [u8; 16], expires_at_epoch: u64) -> Self {
        Self {
            session_id,
            local_ephemeral_key: SigningKey::from_bytes(key_seed),
            local_nonce,
            expires_at_epoch,
        }
    }

    pub fn generate_payload(&self, rendezvous: &str) -> PairingPayload {
        PairingPayload {
            session_id: self.session_id,
            ephemeral_pubkey: self.local_ephemeral_key.verifying_key().to_bytes(),
            nonce: self.local_nonce,
            expires_at_epoch: self.expires_at_epoch,
            rendezvous: rendezvous.to_string(),
        }
    }

    pub fn complete_pairing(
        &self,
        master: &NexMasterIdentity,
        candidate_device_pubkey: &[u8; 32],
        candidate_ephemeral_pk: &[u8; 32],
        candidate_nonce: &[u8; 16],
        expected_sas: u32,
        current_epoch: u64,
        validity_duration: u64,
    ) -> Result<(u32, DeviceCertificate), String> {
        if current_epoch > self.expires_at_epoch {
            return Err("PairingSessionExpired".into());
        }

        let calculated_sas = compute_sas_code(
            &self.local_ephemeral_key.verifying_key().to_bytes(),
            candidate_ephemeral_pk,
            &self.local_nonce,
            candidate_nonce,
        );

        if calculated_sas != expected_sas {
            return Err("SasMismatch".into());
        }

        let cert = master.issue_device_certificate(
            candidate_device_pubkey,
            current_epoch,
            current_epoch + validity_duration,
        )?;

        Ok((calculated_sas, cert))
    }
}
