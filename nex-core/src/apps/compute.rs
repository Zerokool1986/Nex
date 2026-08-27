use std::collections::{BTreeMap, VecDeque};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeJobDescriptor {
    pub job_id: [u8; 32],
    pub wasm_bytecode_hash: [u8; 32],
    pub input_object_ids: Vec<[u8; 32]>,
    pub fuel_limit: u64,
    pub memory_limit_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeResult {
    pub job_id: [u8; 32],
    pub output_bytes: Vec<u8>,
    pub fuel_consumed: u64,
    pub result_commitment: [u8; 32],
}

impl ComputeResult {
    pub fn compute_commitment(job_id: &[u8; 32], output_bytes: &[u8], fuel_consumed: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/COMPUTE_RESULT/v1");
        hasher.update(job_id);
        hasher.update(output_bytes);
        hasher.update(&fuel_consumed.to_be_bytes());
        hasher.finalize().into()
    }

    pub fn new(job_id: [u8; 32], output_bytes: Vec<u8>, fuel_consumed: u64) -> Self {
        let result_commitment = Self::compute_commitment(&job_id, &output_bytes, fuel_consumed);
        Self {
            job_id,
            output_bytes,
            fuel_consumed,
            result_commitment,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeError {
    FuelExhausted,
    MemoryLimitExceeded,
    InvalidBytecode,
    ExecutionTrap(String),
}

pub struct ComputeEngine;

impl ComputeEngine {
    /// Executes a deterministic compute kernel with strict fuel and memory isolation
    pub fn execute_kernel(
        job: &ComputeJobDescriptor,
        bytecode: &[u8],
        inputs: &[Vec<u8>],
    ) -> Result<ComputeResult, ComputeError> {
        if bytecode.is_empty() {
            return Err(ComputeError::InvalidBytecode);
        }

        // Verify bytecode hash
        let mut hasher = Sha256::new();
        hasher.update(bytecode);
        let actual_hash: [u8; 32] = hasher.finalize().into();
        if actual_hash != job.wasm_bytecode_hash {
            return Err(ComputeError::InvalidBytecode);
        }

        let mut fuel_consumed: u64 = 0;
        let mut output = Vec::new();

        // Deterministic execution loop
        for &op in bytecode {
            fuel_consumed += 10;
            if fuel_consumed > job.fuel_limit {
                return Err(ComputeError::FuelExhausted);
            }

            match op {
                0x01 => {
                    // Identity transform of first input
                    if let Some(first) = inputs.first() {
                        if output.len() + first.len() > job.memory_limit_bytes {
                            return Err(ComputeError::MemoryLimitExceeded);
                        }
                        output.extend_from_slice(first);
                    }
                }
                0x02 => {
                    // Hash combination of all inputs
                    let mut input_hasher = Sha256::new();
                    for inp in inputs {
                        input_hasher.update(inp);
                    }
                    let digest: [u8; 32] = input_hasher.finalize().into();
                    if output.len() + 32 > job.memory_limit_bytes {
                        return Err(ComputeError::MemoryLimitExceeded);
                    }
                    output.extend_from_slice(&digest);
                }
                0x03 => {
                    // Inversion / NOT transform
                    for b in &mut output {
                        *b = !*b;
                    }
                }
                0xFF => {
                    // Explicit trap instruction
                    return Err(ComputeError::ExecutionTrap("Kernel raised manual trap".to_string()));
                }
                _ => {
                    // NOP consumes fuel
                }
            }
        }

        Ok(ComputeResult::new(job.job_id, output, fuel_consumed))
    }
}

pub struct ComputeScheduler {
    pub pending_queue: VecDeque<ComputeJobDescriptor>,
    pub completed_jobs: BTreeMap<[u8; 32], (ComputeResult, [u8; 32])>, // job_id -> (result, worker_id)
}

impl ComputeScheduler {
    pub fn new() -> Self {
        Self {
            pending_queue: VecDeque::new(),
            completed_jobs: BTreeMap::new(),
        }
    }

    pub fn submit_job(&mut self, job: ComputeJobDescriptor) {
        self.pending_queue.push_back(job);
    }

    pub fn dispatch_job(&mut self) -> Option<ComputeJobDescriptor> {
        self.pending_queue.pop_front()
    }

    pub fn record_result(&mut self, job_id: [u8; 32], worker: [u8; 32], result: ComputeResult) {
        self.completed_jobs.insert(job_id, (result, worker));
    }

    pub fn get_result(&self, job_id: &[u8; 32]) -> Option<&ComputeResult> {
        self.completed_jobs.get(job_id).map(|(res, _)| res)
    }
}
