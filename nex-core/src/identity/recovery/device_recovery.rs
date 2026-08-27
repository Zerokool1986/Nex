use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use rand::RngCore;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::identity::types::{ActorID, KeyType, DeviceCertificate};
use crate::identity::master::NexMasterIdentity;
use crate::identity::verifier::derive_actor_id;
use crate::identity::recovery::shamir::{GuardianShare, split_secret, combine_shares};
use crate::identity::recovery::ceremony::SocialRecoveryCeremony;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardianFactorType {
    EmergencyPaperKey,
    FamilyGuardian,
    TrustedPeer,
    SecondaryDevice,
    EncryptedVault,
}

impl GuardianFactorType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::EmergencyPaperKey => "Emergency Safety Key (Paper / Safe)",
            Self::FamilyGuardian => "Family Living Circle Guardian",
            Self::TrustedPeer => "Trusted Social Guardian",
            Self::SecondaryDevice => "Secondary Hardware Device",
            Self::EncryptedVault => "Decentralized Encrypted Vault",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardianStatus {
    Configured,
    Escrowed,
    Verified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianRecord {
    pub guardian_index: u8,
    pub name: String,
    pub factor_type: GuardianFactorType,
    pub assigned_actor_id: Option<ActorID>,
    pub status: GuardianStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub root_actor_id: ActorID,
    pub threshold: u8,
    pub total_shares: u8,
    pub created_epoch: u64,
    pub time_lock_epochs: u64,
    pub guardians: Vec<GuardianRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRecoveryResult {
    pub root_actor_id: ActorID,
    pub replacement_device_actor_id: ActorID,
    pub replacement_certificate: DeviceCertificate,
    pub revoked_device_actor_id: Option<ActorID>,
    pub recovery_epoch: u64,
}

pub struct DeviceRecoveryWorkflow;

impl DeviceRecoveryWorkflow {
    /// Establishes the authoritative 3-of-5 recovery configuration for a master root identity.
    pub fn setup_3_of_5_recovery(
        master_seed: &[u8; 32],
        epoch: u64,
        guardian_labels: Option<[&str; 5]>,
        time_lock_epochs: u64,
    ) -> Result<(RecoveryPlan, Vec<GuardianShare>), String> {
        let master = NexMasterIdentity::from_seed(master_seed);
        let root_actor_id = master.root_actor_id;

        // Generate random coefficients for degree (3 - 1) = 2 polynomial over GF(2^8)
        let mut rng = StdRng::from_entropy();
        let mut random_coefficients = Vec::with_capacity(32);
        for _ in 0..32 {
            let mut coeffs = vec![0u8; 2];
            rng.fill_bytes(&mut coeffs);
            random_coefficients.push(coeffs);
        }

        let shares = split_secret(master_seed, 3, 5, epoch, &random_coefficients)?;

        let default_names = [
            "Emergency Master Safety Key",
            "Amy (Family Guardian)",
            "Bob (Trusted Friend)",
            "MacBook Pro (Hardware Device)",
            "Sovereign Decentralized Vault",
        ];

        let names = guardian_labels.unwrap_or(default_names);

        let factor_types = [
            GuardianFactorType::EmergencyPaperKey,
            GuardianFactorType::FamilyGuardian,
            GuardianFactorType::TrustedPeer,
            GuardianFactorType::SecondaryDevice,
            GuardianFactorType::EncryptedVault,
        ];

        let mut guardians = Vec::with_capacity(5);
        for i in 0..5 {
            guardians.push(GuardianRecord {
                guardian_index: (i + 1) as u8,
                name: names[i].to_string(),
                factor_type: factor_types[i],
                assigned_actor_id: None,
                status: GuardianStatus::Verified,
            });
        }

        let plan = RecoveryPlan {
            root_actor_id,
            threshold: 3,
            total_shares: 5,
            created_epoch: epoch,
            time_lock_epochs,
            guardians,
        };

        Ok((plan, shares))
    }

    /// Initializes a social recovery ceremony for an unavailable/lost device.
    pub fn start_ceremony(
        target_root_actor: ActorID,
        time_lock_until_epoch: u64,
    ) -> SocialRecoveryCeremony {
        let mut ceremony_id = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut ceremony_id);
        SocialRecoveryCeremony::new(ceremony_id, target_root_actor, 3, time_lock_until_epoch)
    }

    /// Executes complete recovery: validates quorum, reconstructs original identity,
    /// verifies zero ActorID drift, issues replacement certificate for Device B,
    /// and revokes lost Device A into the CRL.
    pub fn execute_device_recovery(
        ceremony: &SocialRecoveryCeremony,
        replacement_device_pubkey: &[u8; 32],
        lost_device_actor_id: Option<ActorID>,
        current_epoch: u64,
        crl: &mut BTreeSet<ActorID>,
    ) -> Result<DeviceRecoveryResult, String> {
        // 1. Finalize ceremony to reconstruct master seed
        let reconstructed_seed = ceremony.finalize_recovery(current_epoch)?;

        // 2. Reconstruct master identity
        let master = NexMasterIdentity::from_seed(&reconstructed_seed);

        // 3. Assert zero ActorID drift
        if master.root_actor_id != ceremony.target_root_actor {
            return Err("RecoveredActorIdMismatch".into());
        }

        // 4. Issue certificate for replacement device (Device B)
        let not_before = current_epoch;
        let expires_at = current_epoch + 100_000; // 100,000 epochs valid
        let replacement_cert = master.issue_device_certificate(
            replacement_device_pubkey,
            not_before,
            expires_at,
        )?;

        let replacement_device_actor = derive_actor_id(KeyType::Ed25519, replacement_device_pubkey);

        // 5. Revoke lost Device A if specified
        if let Some(lost_actor) = lost_device_actor_id {
            master.revoke_device(crl, lost_actor);
        }

        Ok(DeviceRecoveryResult {
            root_actor_id: master.root_actor_id,
            replacement_device_actor_id: replacement_device_actor,
            replacement_certificate: replacement_cert,
            revoked_device_actor_id: lost_device_actor_id,
            recovery_epoch: current_epoch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_recovery_workflow_lifecycle() {
        let master_seed = [0x42; 32];
        let master = NexMasterIdentity::from_seed(&master_seed);
        let root_actor = master.root_actor_id;

        // 1. Setup 3-of-5 recovery
        let (plan, shares) = DeviceRecoveryWorkflow::setup_3_of_5_recovery(&master_seed, 100, None, 0).unwrap();
        assert_eq!(plan.threshold, 3);
        assert_eq!(plan.total_shares, 5);
        assert_eq!(shares.len(), 5);

        // 2. Device A is lost
        let device_a_pubkey = [0x11; 32];
        let device_a_actor = derive_actor_id(KeyType::Ed25519, &device_a_pubkey);

        // 3. Start ceremony on Device B
        let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(root_actor, 0);

        // Submit 3 shares (Shares 1, 2, 4)
        ceremony.submit_share(shares[0].clone()).unwrap();
        ceremony.submit_share(shares[1].clone()).unwrap();
        ceremony.submit_share(shares[3].clone()).unwrap();

        // 4. Replacement Device B details
        let device_b_pubkey = [0x22; 32];
        let mut crl = BTreeSet::new();

        let recovery_res = DeviceRecoveryWorkflow::execute_device_recovery(
            &ceremony,
            &device_b_pubkey,
            Some(device_a_actor),
            105,
            &mut crl,
        ).unwrap();

        // 5. Assert identity continuity
        assert_eq!(recovery_res.root_actor_id, root_actor, "Root ActorID must remain invariant");
        assert!(crl.contains(&device_a_actor), "Lost Device A must be in CRL");
        assert_eq!(recovery_res.replacement_certificate.master_actor_id, root_actor);
        assert_eq!(recovery_res.replacement_certificate.device_actor_id, recovery_res.replacement_device_actor_id);
    }
}
