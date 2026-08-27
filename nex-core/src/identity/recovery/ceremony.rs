use std::collections::BTreeSet;
use crate::identity::types::ActorID;
use crate::identity::recovery::shamir::{GuardianShare, combine_shares};

pub struct SocialRecoveryCeremony {
    pub ceremony_id: [u8; 16],
    pub target_root_actor: ActorID,
    pub threshold: u8,
    pub time_lock_until_epoch: u64,
    pub collected_shares: Vec<GuardianShare>,
    pub is_canceled: bool,
    seen_indices: BTreeSet<u8>,
}

impl SocialRecoveryCeremony {
    pub fn new(
        ceremony_id: [u8; 16],
        target_root_actor: ActorID,
        threshold: u8,
        time_lock_until_epoch: u64,
    ) -> Self {
        Self {
            ceremony_id,
            target_root_actor,
            threshold,
            time_lock_until_epoch,
            collected_shares: Vec::new(),
            is_canceled: false,
            seen_indices: BTreeSet::new(),
        }
    }

    pub fn submit_share(&mut self, share: GuardianShare) -> Result<usize, String> {
        if self.is_canceled {
            return Err("CeremonyCanceledByOwner".into());
        }
        if self.seen_indices.contains(&share.guardian_index) {
            return Err("DuplicateGuardianShare".into());
        }

        self.seen_indices.insert(share.guardian_index);
        self.collected_shares.push(share);
        Ok(self.collected_shares.len())
    }

    pub fn cancel_ceremony(&mut self) {
        self.is_canceled = true;
        self.collected_shares.clear();
    }

    pub fn finalize_recovery(&self, current_epoch: u64) -> Result<[u8; 32], String> {
        if self.is_canceled {
            return Err("CeremonyCanceledByOwner".into());
        }
        if self.collected_shares.len() < self.threshold as usize {
            return Err("InsufficientSharesForQuorum".into());
        }

        // Time-lock enforcement: unless all total_shares are presented, must wait until time-lock passes
        if current_epoch < self.time_lock_until_epoch && self.collected_shares.len() < self.collected_shares[0].total_shares as usize {
            return Err("TimeLockActiveWaitRequired".into());
        }

        combine_shares(&self.collected_shares, self.threshold)
    }
}
