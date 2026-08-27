use std::collections::BTreeMap;
use crate::model::ActorID;

#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub capacity: u32,
    pub refill_rate_per_sec: u32,
    pub current_tokens: u32,
    pub last_refill_epoch: u64,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate_per_sec: u32, current_epoch: u64) -> Self {
        Self {
            capacity,
            refill_rate_per_sec,
            current_tokens: capacity,
            last_refill_epoch: current_epoch,
        }
    }

    pub fn try_consume(&mut self, current_epoch: u64, tokens_needed: u32) -> bool {
        if current_epoch > self.last_refill_epoch {
            let elapsed = current_epoch - self.last_refill_epoch;
            let refill_amount = (elapsed as u32).saturating_mul(self.refill_rate_per_sec);
            self.current_tokens = (self.current_tokens + refill_amount).min(self.capacity);
            self.last_refill_epoch = current_epoch;
        }

        if self.current_tokens >= tokens_needed {
            self.current_tokens -= tokens_needed;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PeerRateLimiter {
    pub default_capacity: u32,
    pub default_refill_rate: u32,
    pub peer_buckets: BTreeMap<ActorID, TokenBucket>,
}

impl PeerRateLimiter {
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            default_capacity: capacity,
            default_refill_rate: refill_rate,
            peer_buckets: BTreeMap::new(),
        }
    }

    pub fn check_and_consume(&mut self, peer: &ActorID, current_epoch: u64, tokens: u32) -> bool {
        let bucket = self.peer_buckets.entry(*peer).or_insert_with(|| {
            TokenBucket::new(self.default_capacity, self.default_refill_rate, current_epoch)
        });
        bucket.try_consume(current_epoch, tokens)
    }
}
