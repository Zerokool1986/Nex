use crate::model::*;

pub const NEX_SERIALIZATION_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationError {
    LengthOverflow,
    UnsortedParents,
    DuplicateParents,
    UnsortedFrontier,
    DuplicateFrontier,
}

pub trait CanonicalSerialize {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError>;
}

impl CanonicalSerialize for u64 {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        buf.extend_from_slice(&self.to_le_bytes()); // Strict little-endian
        Ok(())
    }
}

impl CanonicalSerialize for [u8; 32] {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        buf.extend_from_slice(self);
        Ok(())
    }
}

impl CanonicalSerialize for Vec<u8> {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        if self.len() > u32::MAX as usize {
            return Err(SerializationError::LengthOverflow);
        }
        buf.extend_from_slice(&(self.len() as u32).to_le_bytes()); // 32-bit length prefix
        buf.extend_from_slice(self);
        Ok(())
    }
}

impl CanonicalSerialize for CrdtPayload {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        match self {
            CrdtPayload::AddLWW { id, value } => {
                buf.push(0x01); // Explicit Discriminant
                id.canonical_serialize(buf)?;
                value.canonical_serialize(buf)?;
            }
            CrdtPayload::RemoveLWW { id } => {
                buf.push(0x02); // Explicit Discriminant
                id.canonical_serialize(buf)?;
            }
            CrdtPayload::Tombstone { id } => {
                buf.push(0x03); // Explicit Discriminant
                id.canonical_serialize(buf)?;
            }
        }
        Ok(())
    }
}

impl CanonicalSerialize for MutationBody {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        buf.push(NEX_SERIALIZATION_VERSION);
        self.author.canonical_serialize(buf)?;

        // Strict Parent Validation: must be strictly sorted without duplicates
        for i in 1..self.parents.len() {
            if self.parents[i - 1] == self.parents[i] {
                return Err(SerializationError::DuplicateParents);
            }
            if self.parents[i - 1] > self.parents[i] {
                return Err(SerializationError::UnsortedParents);
            }
        }

        if self.parents.len() > u32::MAX as usize {
            return Err(SerializationError::LengthOverflow);
        }
        buf.extend_from_slice(&(self.parents.len() as u32).to_le_bytes());
        for p in &self.parents {
            p.canonical_serialize(buf)?;
        }

        self.lamport.canonical_serialize(buf)?;
        self.epoch.canonical_serialize(buf)?;
        buf.push(if self.is_resurrect { 1 } else { 0 }); // Explicit boolean

        self.payload.canonical_serialize(buf)?;
        Ok(())
    }
}

impl CanonicalSerialize for Mutation {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.body.canonical_serialize(buf)
    }
}

impl CanonicalSerialize for StateEncoding {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        buf.push(NEX_SERIALIZATION_VERSION);
        self.mutation_id.canonical_serialize(buf)?;
        self.lamport.canonical_serialize(buf)?;
        self.epoch.canonical_serialize(buf)?;
        buf.push(if self.is_resurrect { 1 } else { 0 });
        self.payload.canonical_serialize(buf)?;
        Ok(())
    }
}

impl CanonicalSerialize for Boundary {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.max_epoch.canonical_serialize(buf)?;
        self.max_lamport.canonical_serialize(buf)?;
        Ok(())
    }
}

impl CanonicalSerialize for CheckpointBody {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        buf.push(NEX_SERIALIZATION_VERSION);
        self.state_root.canonical_serialize(buf)?;
        self.causal_root.canonical_serialize(buf)?;
        self.admission_root.canonical_serialize(buf)?;

        // Strict Frontier Validation: must be strictly sorted without duplicates
        for i in 1..self.frontier.len() {
            if self.frontier[i - 1] == self.frontier[i] {
                return Err(SerializationError::DuplicateFrontier);
            }
            if self.frontier[i - 1] > self.frontier[i] {
                return Err(SerializationError::UnsortedFrontier);
            }
        }

        if self.frontier.len() > u32::MAX as usize {
            return Err(SerializationError::LengthOverflow);
        }
        buf.extend_from_slice(&(self.frontier.len() as u32).to_le_bytes());
        for f in &self.frontier {
            f.canonical_serialize(buf)?;
        }
        self.boundary.canonical_serialize(buf)?;
        Ok(())
    }
}

impl CanonicalSerialize for Checkpoint {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.body.canonical_serialize(buf)
    }
}

impl CanonicalSerialize for PublicStatement {
    fn canonical_serialize(&self, buf: &mut Vec<u8>) -> Result<(), SerializationError> {
        buf.push(NEX_SERIALIZATION_VERSION);
        buf.extend_from_slice(&self.semantic_abi_version.to_le_bytes());
        self.input_commitment.canonical_serialize(buf)?;
        self.frontier_commitment.canonical_serialize(buf)?;
        self.claimed_state_root.canonical_serialize(buf)?;
        self.claimed_causal_root.canonical_serialize(buf)?;
        self.claimed_admission_root.canonical_serialize(buf)?;
        self.claimed_boundary.canonical_serialize(buf)?;
        self.claimed_checkpoint_id.canonical_serialize(buf)?;
        self.initial_smt_root.canonical_serialize(buf)?;
        self.final_smt_root.canonical_serialize(buf)?;
        buf.extend_from_slice(&self.mutations_admitted.to_le_bytes());
        Ok(())
    }
}
