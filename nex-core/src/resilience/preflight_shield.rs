use crate::model::Checkpoint;
use crate::hash::hash_checkpoint_body;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreFlightError {
    ImageIdMismatch,
    AbiVersionMismatch(u32),
    EmptySeal,
    SealOversized(usize),
    CheckpointIdPreimageMismatch,
}

pub const MAX_SEAL_BYTES: usize = 256 * 1024; // 256 KB

#[derive(Debug, Clone)]
pub struct PreFlightShield {
    pub expected_image_id: [u8; 32],
}

impl PreFlightShield {
    pub fn new(expected_image_id: [u8; 32]) -> Self {
        Self { expected_image_id }
    }

    /// Performs fast O(1) semantic validation (< 1 microsecond) before invoking expensive STARK cryptographic verifier
    pub fn validate_proof_preflight(
        &self,
        image_id: &[u8; 32],
        abi_version: u32,
        seal_bytes: &[u8],
        checkpoint: &Checkpoint,
    ) -> Result<(), PreFlightError> {
        // 1. Fast ImageID match check
        if *image_id != self.expected_image_id {
            return Err(PreFlightError::ImageIdMismatch);
        }

        // 2. ABI compatibility check
        if abi_version != 1 {
            return Err(PreFlightError::AbiVersionMismatch(abi_version));
        }

        // 3. Seal bounds check
        if seal_bytes.is_empty() {
            return Err(PreFlightError::EmptySeal);
        }
        if seal_bytes.len() > MAX_SEAL_BYTES {
            return Err(PreFlightError::SealOversized(seal_bytes.len()));
        }

        // 4. Fast Checkpoint ID preimage check
        let derived_cp_id = hash_checkpoint_body(&checkpoint.body);
        if checkpoint.id != derived_cp_id {
            return Err(PreFlightError::CheckpointIdPreimageMismatch);
        }

        Ok(())
    }
}
