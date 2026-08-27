use serde::{Deserialize, Serialize};

pub const FRAME_MAGIC: [u8; 2] = [0x4E, 0x58]; // 'NX'
pub const HEADER_LEN: usize = 11; // 2B Magic + 2B Tag + 1B Flags + 4B Len + 4B CRC32 - wait: 2+2+1+4+4 = 13 bytes

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportGuarantee {
    ReliableStream,
    UnreliableDatagram,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportPacket {
    pub transport_tag: u16,
    pub source_address: Vec<u8>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    SendFailed(String),
    CorruptedFrame(String),
    MtuExceeded { payload_len: usize, mtu: usize },
    IncompleteReassembly,
    UnsupportedTransport(u16),
    NoRoutableTransport,
}

/// Standard IEEE 802.3 CRC32 implementation
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

pub fn encode_frame(transport_tag: u16, flags: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(13 + payload.len());
    frame.extend_from_slice(&FRAME_MAGIC);
    frame.extend_from_slice(&transport_tag.to_be_bytes());
    frame.push(flags);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    let crc = compute_crc32(payload);
    frame.extend_from_slice(&crc.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

pub fn decode_frame(raw: &[u8]) -> Result<(u16, u8, Vec<u8>), TransportError> {
    if raw.len() < 13 {
        return Err(TransportError::CorruptedFrame("Frame too short for header".into()));
    }
    if raw[0..2] != FRAME_MAGIC {
        return Err(TransportError::CorruptedFrame("Invalid magic bytes".into()));
    }

    let transport_tag = u16::from_be_bytes([raw[2], raw[3]]);
    let flags = raw[4];
    let payload_len = u32::from_be_bytes([raw[5], raw[6], raw[7], raw[8]]) as usize;
    let expected_crc = u32::from_be_bytes([raw[9], raw[10], raw[11], raw[12]]);

    if raw.len() < 13 + payload_len {
        return Err(TransportError::CorruptedFrame("Payload truncated".into()));
    }

    let payload = &raw[13..13 + payload_len];
    let actual_crc = compute_crc32(payload);
    if actual_crc != expected_crc {
        return Err(TransportError::CorruptedFrame(format!(
            "CRC32 mismatch: expected 0x{:08X}, got 0x{:08X}", expected_crc, actual_crc
        )));
    }

    Ok((transport_tag, flags, payload.to_vec()))
}
