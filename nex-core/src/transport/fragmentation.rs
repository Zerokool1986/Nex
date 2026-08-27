use std::collections::BTreeMap;
use crate::transport::types::TransportError;

pub const CHUNK_HEADER_LEN: usize = 36; // 32B msg_id + 2B index + 2B total
pub const MAX_IN_FLIGHT_REASSEMBLIES: usize = 128;
pub const DEFAULT_REASSEMBLY_TTL_EPOCHS: u64 = 30;

#[derive(Debug, Clone, Default)]
pub struct FragmentationReassembler {
    /// In-flight message buffer: message_id -> (total_chunks, created_epoch, map(chunk_index -> chunk_payload))
    pub in_flight: BTreeMap<[u8; 32], (u16, u64, BTreeMap<u16, Vec<u8>>)>,
}

impl FragmentationReassembler {
    pub fn new() -> Self {
        Self { in_flight: BTreeMap::new() }
    }

    /// Ingests a raw chunk payload. Returns Some(complete_payload) if all chunks for the message have arrived.
    pub fn ingest_chunk(&mut self, raw_chunk: &[u8]) -> Result<Option<Vec<u8>>, TransportError> {
        self.ingest_chunk_with_epoch(raw_chunk, 0)
    }

    /// Ingests a raw chunk payload with epoch stamping for timeout pruning.
    pub fn ingest_chunk_with_epoch(&mut self, raw_chunk: &[u8], current_epoch: u64) -> Result<Option<Vec<u8>>, TransportError> {
        if raw_chunk.len() < CHUNK_HEADER_LEN {
            return Err(TransportError::CorruptedFrame("Chunk smaller than chunk header".into()));
        }

        let mut msg_id = [0u8; 32];
        msg_id.copy_from_slice(&raw_chunk[0..32]);
        let chunk_index = u16::from_be_bytes([raw_chunk[32], raw_chunk[33]]);
        let total_chunks = u16::from_be_bytes([raw_chunk[34], raw_chunk[35]]);
        let chunk_payload = &raw_chunk[36..];

        if total_chunks == 0 || chunk_index >= total_chunks {
            return Err(TransportError::CorruptedFrame("Invalid chunk index or total chunks".into()));
        }

        if total_chunks == 1 {
            return Ok(Some(chunk_payload.to_vec()));
        }

        if self.in_flight.len() >= MAX_IN_FLIGHT_REASSEMBLIES && !self.in_flight.contains_key(&msg_id) {
            // Prune stale streams first
            self.prune_stale_streams(current_epoch, DEFAULT_REASSEMBLY_TTL_EPOCHS);
            if self.in_flight.len() >= MAX_IN_FLIGHT_REASSEMBLIES {
                return Err(TransportError::SendFailed("Reassembly buffer capacity exceeded".into()));
            }
        }

        let entry = self.in_flight.entry(msg_id).or_insert_with(|| (total_chunks, current_epoch, BTreeMap::new()));
        if entry.0 != total_chunks {
            return Err(TransportError::CorruptedFrame("Mismatched total chunks for message ID".into()));
        }

        entry.2.insert(chunk_index, chunk_payload.to_vec());

        if entry.2.len() == (total_chunks as usize) {
            let (_, _, chunks_map) = self.in_flight.remove(&msg_id).unwrap();
            let mut complete_payload = Vec::new();
            for i in 0..total_chunks {
                if let Some(chunk) = chunks_map.get(&i) {
                    complete_payload.extend_from_slice(chunk);
                } else {
                    return Err(TransportError::IncompleteReassembly);
                }
            }
            Ok(Some(complete_payload))
        } else {
            Ok(None)
        }
    }

    /// Prunes in-flight reassembly streams that have exceeded TTL
    pub fn prune_stale_streams(&mut self, current_epoch: u64, ttl_epochs: u64) -> usize {
        let mut stale_ids = Vec::new();
        for (id, (_, created_epoch, _)) in &self.in_flight {
            if current_epoch.saturating_sub(*created_epoch) > ttl_epochs {
                stale_ids.push(*id);
            }
        }

        let count = stale_ids.len();
        for id in stale_ids {
            self.in_flight.remove(&id);
        }
        count
    }
}

/// Splits a large payload into individual chunks fitting within the specified MTU
pub fn fragment_payload(message_id: [u8; 32], payload: &[u8], mtu: usize) -> Result<Vec<Vec<u8>>, TransportError> {
    if mtu <= CHUNK_HEADER_LEN {
        return Err(TransportError::MtuExceeded { payload_len: payload.len(), mtu });
    }

    let chunk_cap = mtu - CHUNK_HEADER_LEN;
    let total_chunks = if payload.is_empty() {
        1
    } else {
        ((payload.len() + chunk_cap - 1) / chunk_cap) as u16
    };

    let mut chunks = Vec::with_capacity(total_chunks as usize);
    for chunk_idx in 0..total_chunks {
        let start = (chunk_idx as usize) * chunk_cap;
        let end = (start + chunk_cap).min(payload.len());
        let chunk_slice = if payload.is_empty() { &[] } else { &payload[start..end] };

        let mut chunk_raw = Vec::with_capacity(CHUNK_HEADER_LEN + chunk_slice.len());
        chunk_raw.extend_from_slice(&message_id);
        chunk_raw.extend_from_slice(&chunk_idx.to_be_bytes());
        chunk_raw.extend_from_slice(&total_chunks.to_be_bytes());
        chunk_raw.extend_from_slice(chunk_slice);
        chunks.push(chunk_raw);
    }

    Ok(chunks)
}
