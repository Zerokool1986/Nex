use std::collections::{BTreeMap, BTreeSet};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardDescriptor {
    pub chunk_hash: [u8; 32],
    pub shard_index: u8,
    pub total_shards: u8,
    pub data: Vec<u8>,
}

impl ShardDescriptor {
    pub fn shard_id(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/SHARD_ID/v1");
        hasher.update(&self.chunk_hash);
        hasher.update(&[self.shard_index]);
        hasher.finalize().into()
    }
}

pub struct ErasureCoder;

impl ErasureCoder {
    /// Splits payload into `k` data shards and 1 parity shard (XOR combination) for 1-fault tolerance
    pub fn split(payload: &[u8], data_shards: usize) -> Vec<ShardDescriptor> {
        let total_shards = (data_shards + 1) as u8;
        let original_len = payload.len();
        let chunk_size = (original_len + data_shards - 1) / data_shards;
        
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let chunk_hash: [u8; 32] = hasher.finalize().into();

        let mut shards = Vec::new();
        let mut parity_data = vec![0u8; chunk_size];

        for i in 0..data_shards {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(original_len);
            let mut slice = vec![0u8; chunk_size];
            if start < original_len {
                slice[..(end - start)].copy_from_slice(&payload[start..end]);
            }
            for j in 0..chunk_size {
                parity_data[j] ^= slice[j];
            }
            shards.push(ShardDescriptor {
                chunk_hash,
                shard_index: i as u8,
                total_shards,
                data: slice,
            });
        }

        // Parity shard
        shards.push(ShardDescriptor {
            chunk_hash,
            shard_index: data_shards as u8,
            total_shards,
            data: parity_data,
        });

        shards
    }

    /// Reconstructs original payload from any `k` available shards
    pub fn reconstruct(
        available_shards: &[ShardDescriptor],
        data_shards: usize,
        original_len: usize,
    ) -> Result<Vec<u8>, String> {
        if available_shards.len() < data_shards {
            return Err("Insufficient shards for reconstruction".to_string());
        }

        let chunk_size = (original_len + data_shards - 1) / data_shards;
        let mut shard_map: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
        for s in available_shards {
            shard_map.insert(s.shard_index, s.data.clone());
        }

        // Check if all data shards (0..data_shards) are directly present
        let mut all_data_present = true;
        for i in 0..data_shards {
            if !shard_map.contains_key(&(i as u8)) {
                all_data_present = false;
                break;
            }
        }

        if all_data_present {
            let mut payload = Vec::with_capacity(original_len);
            for i in 0..data_shards {
                let slice = shard_map.get(&(i as u8)).unwrap();
                payload.extend_from_slice(slice);
            }
            payload.truncate(original_len);
            return Ok(payload);
        }

        // If one data shard is missing but parity shard (index = data_shards) is present
        if let Some(parity) = shard_map.get(&(data_shards as u8)) {
            let mut missing_index = None;
            for i in 0..data_shards {
                if !shard_map.contains_key(&(i as u8)) {
                    if missing_index.is_some() {
                        return Err("More than 1 data shard missing, cannot reconstruct with 1 parity shard".to_string());
                    }
                    missing_index = Some(i);
                }
            }

            if let Some(missing_i) = missing_index {
                let mut recovered_slice = parity.clone();
                for i in 0..data_shards {
                    if i != missing_i {
                        let other = shard_map.get(&(i as u8)).unwrap();
                        for j in 0..chunk_size {
                            recovered_slice[j] ^= other[j];
                        }
                    }
                }
                shard_map.insert(missing_i as u8, recovered_slice);

                let mut payload = Vec::with_capacity(original_len);
                for i in 0..data_shards {
                    let slice = shard_map.get(&(i as u8)).unwrap();
                    payload.extend_from_slice(slice);
                }
                payload.truncate(original_len);
                return Ok(payload);
            }
        }

        Err("Reconstruction failed".to_string())
    }
}

pub struct ProofOfRetrievability;

impl ProofOfRetrievability {
    pub fn prove_storage(shard_data: &[u8], nonce: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/POR/v1");
        hasher.update(&nonce.to_be_bytes());
        hasher.update(shard_data);
        hasher.finalize().into()
    }

    pub fn verify_proof(proof: [u8; 32], shard_data: &[u8], nonce: u64) -> bool {
        let expected = Self::prove_storage(shard_data, nonce);
        expected == proof
    }
}

pub struct BilateralCreditLedger {
    pub balances: BTreeMap<String, i64>,
}

impl BilateralCreditLedger {
    pub fn new() -> Self {
        Self {
            balances: BTreeMap::new(),
        }
    }

    pub fn record_transfer(
        &mut self,
        peer: [u8; 32],
        delta_bytes: i64,
        max_debt_ceiling: i64,
    ) -> Result<i64, String> {
        let key = hex::encode(peer);
        let current = self.balances.get(&key).copied().unwrap_or(0);
        let new_balance = current + delta_bytes;

        if new_balance < -max_debt_ceiling {
            return Err("Debt ceiling exceeded for peer".to_string());
        }

        self.balances.insert(key, new_balance);
        Ok(new_balance)
    }

    pub fn get_balance(&self, peer: &[u8; 32]) -> i64 {
        let key = hex::encode(peer);
        self.balances.get(&key).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardHealthStatus {
    Healthy,
    Degraded,
    Critical,
}

pub struct ShardHealthAuditor {
    pub chunk_providers: BTreeMap<[u8; 32], BTreeSet<[u8; 32]>>,
}

impl ShardHealthAuditor {
    pub fn new() -> Self {
        Self {
            chunk_providers: BTreeMap::new(),
        }
    }

    pub fn register_provider(&mut self, chunk_hash: [u8; 32], provider: [u8; 32]) {
        self.chunk_providers.entry(chunk_hash).or_default().insert(provider);
    }

    pub fn unregister_provider(&mut self, chunk_hash: &[u8; 32], provider: &[u8; 32]) {
        if let Some(set) = self.chunk_providers.get_mut(chunk_hash) {
            set.remove(provider);
        }
    }

    pub fn audit_health(&self, chunk_hash: &[u8; 32], required_min: usize, ideal_total: usize) -> ShardHealthStatus {
        let count = self.chunk_providers.get(chunk_hash).map_or(0, |set| set.len());
        if count >= ideal_total {
            ShardHealthStatus::Healthy
        } else if count >= required_min {
            ShardHealthStatus::Degraded
        } else {
            ShardHealthStatus::Critical
        }
    }
}
