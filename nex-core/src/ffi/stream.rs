use sha2::{Sha256, Digest};
use crate::apps::drive::CasChunkStore;
use crate::ffi::handle::{NEX_ERR_CAS_CORRUPTION, NEX_ERR_INTERNAL_ERROR};

pub const CAS_STREAM_MAGIC: u32 = 0x4E584353; // "NXCS"
pub const OP_PUT_CHUNK: u16 = 0x0001;
pub const OP_GET_CHUNK: u16 = 0x0002;
pub const CAS_HEADER_SIZE: usize = 48; // 16 bytes fixed metadata + 32 bytes SHA-256 digest

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasStreamHeader {
    pub magic: u32,
    pub opcode: u16,
    pub flags: u16,
    pub payload_len: u32,
    pub reserved: u32,
    pub expected_digest: [u8; 32],
}

impl CasStreamHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, i32> {
        if bytes.len() < CAS_HEADER_SIZE {
            return Err(NEX_ERR_INTERNAL_ERROR);
        }

        let magic = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        if magic != CAS_STREAM_MAGIC {
            return Err(NEX_ERR_INTERNAL_ERROR);
        }

        let opcode = u16::from_be_bytes(bytes[4..6].try_into().unwrap());
        let flags = u16::from_be_bytes(bytes[6..8].try_into().unwrap());
        let payload_len = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let reserved = u32::from_be_bytes(bytes[12..16].try_into().unwrap());

        let mut expected_digest = [0u8; 32];
        expected_digest.copy_from_slice(&bytes[16..48]);

        Ok(Self {
            magic,
            opcode,
            flags,
            payload_len,
            reserved,
            expected_digest,
        })
    }

    pub fn serialize(&self) -> [u8; CAS_HEADER_SIZE] {
        let mut buf = [0u8; CAS_HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_be_bytes());
        buf[4..6].copy_from_slice(&self.opcode.to_be_bytes());
        buf[6..8].copy_from_slice(&self.flags.to_be_bytes());
        buf[8..12].copy_from_slice(&self.payload_len.to_be_bytes());
        buf[12..16].copy_from_slice(&self.reserved.to_be_bytes());
        buf[16..48].copy_from_slice(&self.expected_digest);
        buf
    }
}

pub struct CasStreamProcessor;

impl CasStreamProcessor {
    pub fn process_put_chunk(
        cas: &mut CasChunkStore,
        header: &CasStreamHeader,
        payload: &[u8],
    ) -> Result<[u8; 32], i32> {
        if payload.len() != header.payload_len as usize {
            return Err(NEX_ERR_INTERNAL_ERROR);
        }

        let actual_digest: [u8; 32] = Sha256::digest(payload).into();
        if actual_digest != header.expected_digest {
            return Err(NEX_ERR_CAS_CORRUPTION);
        }

        cas.chunks.insert(actual_digest, payload.to_vec());
        Ok(actual_digest)
    }

    pub fn process_get_chunk(
        cas: &CasChunkStore,
        digest: &[u8; 32],
    ) -> Result<Vec<u8>, i32> {
        match cas.get_chunk(digest) {
            Some(data) => Ok(data.clone()),
            None => Err(NEX_ERR_CAS_CORRUPTION),
        }
    }
}
