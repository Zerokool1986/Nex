use std::collections::BTreeMap;
use crate::model::ActorID;

pub const PENALTY_THRESHOLD: u32 = 100;
pub const BASE_JAIL_DURATION_EPOCHS: u64 = 60;

#[derive(Debug, Clone, Default)]
pub struct PeerJail {
    pub penalties: BTreeMap<ActorID, u32>,
    pub jailed_until: BTreeMap<ActorID, u64>,
    pub jail_counts: BTreeMap<ActorID, u32>,
}

impl PeerJail {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records penalty points against a peer. If threshold is exceeded, jailing is triggered.
    pub fn record_penalty(&mut self, peer: &ActorID, points: u32, current_epoch: u64) -> bool {
        let current_points = self.penalties.entry(*peer).or_insert(0);
        *current_points += points;

        if *current_points >= PENALTY_THRESHOLD {
            let count = self.jail_counts.entry(*peer).or_insert(0);
            *count += 1;
            // Progressive escalating jail duration: base * 2^(count - 1)
            let duration = BASE_JAIL_DURATION_EPOCHS.saturating_mul(1 << (count.saturating_sub(1).min(10)));
            self.jailed_until.insert(*peer, current_epoch + duration);
            *current_points = 0; // Reset penalty points
            true
        } else {
            false
        }
    }

    /// Checks if a peer is currently jailed
    pub fn is_jailed(&mut self, peer: &ActorID, current_epoch: u64) -> bool {
        if let Some(&until) = self.jailed_until.get(peer) {
            if current_epoch < until {
                return true;
            } else {
                self.jailed_until.remove(peer);
            }
        }
        false
    }
}
