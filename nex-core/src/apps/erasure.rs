use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

// Standard GF(2^8) primitive polynomial: x^8 + x^4 + x^3 + x^2 + 1 (0x11D)
const GF28_POLY: u16 = 0x11D;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReedSolomonShard {
    pub chunk_hash: [u8; 32],
    pub shard_index: u8,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct GaloisField28 {
    exp_table: [u8; 512],
    log_table: [u8; 256],
}

impl GaloisField28 {
    pub fn new() -> Self {
        let mut exp_table = [0u8; 512];
        let mut log_table = [0u8; 256];
        let mut x: u16 = 1;

        for i in 0..255 {
            exp_table[i] = x as u8;
            exp_table[i + 255] = x as u8;
            log_table[x as usize] = i as u8;
            x <<= 1;
            if (x & 0x100) != 0 {
                x ^= GF28_POLY;
            }
        }
        exp_table[510] = exp_table[0];
        exp_table[511] = exp_table[1];

        Self {
            exp_table,
            log_table,
        }
    }

    #[inline(always)]
    pub fn add(&self, a: u8, b: u8) -> u8 {
        a ^ b
    }

    #[inline(always)]
    pub fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let log_a = self.log_table[a as usize] as usize;
            let log_b = self.log_table[b as usize] as usize;
            self.exp_table[log_a + log_b]
        }
    }

    #[inline(always)]
    pub fn div(&self, a: u8, b: u8) -> u8 {
        assert!(b != 0, "Division by zero in GF(2^8)");
        if a == 0 {
            0
        } else {
            let log_a = self.log_table[a as usize] as usize;
            let log_b = self.log_table[b as usize] as usize;
            let diff = log_a + 255 - log_b;
            self.exp_table[diff]
        }
    }

    #[inline(always)]
    pub fn inv(&self, a: u8) -> u8 {
        assert!(a != 0, "Zero has no inverse in GF(2^8)");
        let log_a = self.log_table[a as usize] as usize;
        self.exp_table[255 - log_a]
    }
}

pub struct ReedSolomonEngine {
    gf: GaloisField28,
}

impl Default for ReedSolomonEngine {
    fn default() -> Self {
        Self {
            gf: GaloisField28::new(),
        }
    }
}

impl ReedSolomonEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generates Cauchy distribution matrix for (K, M)
    fn build_cauchy_matrix(&self, rows: usize, cols: usize) -> Vec<Vec<u8>> {
        let mut matrix = vec![vec![0u8; cols]; rows];
        for r in 0..rows {
            for c in 0..cols {
                let x_i = (r + 1) as u8;
                let y_j = (cols + c + 1) as u8;
                let denom = self.gf.add(x_i, y_j);
                matrix[r][c] = self.gf.inv(denom);
            }
        }
        matrix
    }

    /// Splits payload into K data shards and M parity shards
    pub fn encode(
        &self,
        payload: &[u8],
        data_shards: usize,
        parity_shards: usize,
    ) -> Vec<ReedSolomonShard> {
        assert!(data_shards > 0 && parity_shards > 0);
        let original_len = payload.len();
        let chunk_size = (original_len + data_shards - 1) / data_shards;

        let mut hasher = Sha256::new();
        hasher.update(payload);
        let chunk_hash: [u8; 32] = hasher.finalize().into();

        // 1. Partition data shards
        let mut data_data = vec![vec![0u8; chunk_size]; data_shards];
        for i in 0..data_shards {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(original_len);
            if start < original_len {
                data_data[i][..(end - start)].copy_from_slice(&payload[start..end]);
            }
        }

        // 2. Generate Cauchy parity shards
        let cauchy = self.build_cauchy_matrix(parity_shards, data_shards);
        let mut parity_data = vec![vec![0u8; chunk_size]; parity_shards];

        for p in 0..parity_shards {
            for d in 0..data_shards {
                let coeff = cauchy[p][d];
                for byte_idx in 0..chunk_size {
                    let term = self.gf.mul(coeff, data_data[d][byte_idx]);
                    parity_data[p][byte_idx] = self.gf.add(parity_data[p][byte_idx], term);
                }
            }
        }

        // 3. Assemble shards
        let total_shards = (data_shards + parity_shards) as u8;
        let mut shards = Vec::with_capacity(total_shards as usize);

        for i in 0..data_shards {
            shards.push(ReedSolomonShard {
                chunk_hash,
                shard_index: i as u8,
                data_shards: data_shards as u8,
                parity_shards: parity_shards as u8,
                data: data_data[i].clone(),
            });
        }

        for p in 0..parity_shards {
            shards.push(ReedSolomonShard {
                chunk_hash,
                shard_index: (data_shards + p) as u8,
                data_shards: data_shards as u8,
                parity_shards: parity_shards as u8,
                data: parity_data[p].clone(),
            });
        }

        shards
    }

    /// Inverts a square matrix in GF(2^8) via Gaussian elimination
    fn invert_matrix(&self, mut matrix: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>, String> {
        let n = matrix.len();
        let mut inv = vec![vec![0u8; n]; n];
        for i in 0..n {
            inv[i][i] = 1;
        }

        for i in 0..n {
            // Pivot search
            let mut pivot = i;
            while pivot < n && matrix[pivot][i] == 0 {
                pivot += 1;
            }
            if pivot == n {
                return Err("Singular matrix in GF(2^8) inversion".into());
            }
            if pivot != i {
                matrix.swap(i, pivot);
                inv.swap(i, pivot);
            }

            // Scale pivot row to 1
            let pivot_val = matrix[i][i];
            let pivot_inv = self.gf.inv(pivot_val);
            for j in 0..n {
                matrix[i][j] = self.gf.mul(matrix[i][j], pivot_inv);
                inv[i][j] = self.gf.mul(inv[i][j], pivot_inv);
            }

            // Eliminate column
            for r in 0..n {
                if r != i {
                    let factor = matrix[r][i];
                    if factor != 0 {
                        for c in 0..n {
                            let term = self.gf.mul(factor, matrix[i][c]);
                            matrix[r][c] = self.gf.add(matrix[r][c], term);
                            let inv_term = self.gf.mul(factor, inv[i][c]);
                            inv[r][c] = self.gf.add(inv[r][c], inv_term);
                        }
                    }
                }
            }
        }

        Ok(inv)
    }

    /// Reconstructs original payload from any K available shards
    pub fn decode(
        &self,
        available_shards: &[ReedSolomonShard],
        data_shards: usize,
        parity_shards: usize,
        original_len: usize,
    ) -> Result<Vec<u8>, String> {
        if available_shards.len() < data_shards {
            return Err(format!(
                "Insufficient shards: have {}, need {}",
                available_shards.len(),
                data_shards
            ));
        }

        let chunk_size = (original_len + data_shards - 1) / data_shards;
        let mut shard_map: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
        for s in available_shards {
            shard_map.insert(s.shard_index, s.data.clone());
        }

        // Fast path: all data shards (0..data_shards) present
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
                payload.extend_from_slice(shard_map.get(&(i as u8)).unwrap());
            }
            payload.truncate(original_len);
            return Ok(payload);
        }

        // General path: select first K available shards to build decode matrix
        let cauchy = self.build_cauchy_matrix(parity_shards, data_shards);
        let mut sub_matrix = vec![vec![0u8; data_shards]; data_shards];
        let mut sub_data = vec![vec![0u8; chunk_size]; data_shards];

        let mut count = 0;
        for (&idx, data) in shard_map.iter() {
            if count == data_shards {
                break;
            }
            if (idx as usize) < data_shards {
                // Identity row
                sub_matrix[count][idx as usize] = 1;
            } else {
                // Cauchy parity row
                let p_idx = (idx as usize) - data_shards;
                sub_matrix[count].copy_from_slice(&cauchy[p_idx]);
            }
            sub_data[count] = data.clone();
            count += 1;
        }

        let inv_matrix = self.invert_matrix(sub_matrix)?;
        let mut data_data = vec![vec![0u8; chunk_size]; data_shards];

        for d in 0..data_shards {
            for k in 0..data_shards {
                let coeff = inv_matrix[d][k];
                for byte_idx in 0..chunk_size {
                    let term = self.gf.mul(coeff, sub_data[k][byte_idx]);
                    data_data[d][byte_idx] = self.gf.add(data_data[d][byte_idx], term);
                }
            }
        }

        let mut payload = Vec::with_capacity(original_len);
        for d in 0..data_shards {
            payload.extend_from_slice(&data_data[d]);
        }
        payload.truncate(original_len);

        Ok(payload)
    }
}
