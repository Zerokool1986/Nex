use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NexObject};
use crate::model::{Mutation, MutationID, Checkpoint};
use crate::storage::wal::{WAL_MAGIC, WAL_VERSION};

pub const STATE_DB_MAGIC: &[u8; 4] = b"NEXS";
pub const STATE_DB_VERSION: u8 = 1;

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshotData {
    pub epoch: u64,
    pub lamport: u64,
    pub latest_mutation_id: Option<[u8; 32]>,
    pub frontier: BTreeSet<MutationID>,
    pub crdt_state: BTreeMap<[u8; 32], (Option<Vec<u8>>, u64, u64, MutationID)>,
    pub dag: BTreeMap<MutationID, Mutation>,
    pub object_store: BTreeMap<ObjectID, NexObject>,
    pub checkpoint: Option<Checkpoint>,
}

pub struct StateDbEngine;

impl StateDbEngine {
    pub fn save_snapshot(
        data_dir: impl AsRef<Path>,
        snapshot: &StateSnapshotData,
    ) -> io::Result<()> {
        let dir = data_dir.as_ref();
        fs::create_dir_all(dir)?;

        let tmp_path = dir.join("state.db.tmp");
        let final_path = dir.join("state.db");

        let payload_bytes = bincode::serialize(snapshot)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Serialization error: {:?}", e)))?;

        let checksum = crc32(&payload_bytes);

        // 1. Write and sync staging file
        let mut tmp_file = File::create(&tmp_path)?;
        tmp_file.write_all(STATE_DB_MAGIC)?;
        tmp_file.write_all(&[STATE_DB_VERSION])?;
        tmp_file.write_all(&checksum.to_le_bytes())?;
        tmp_file.write_all(&(payload_bytes.len() as u64).to_le_bytes())?;
        tmp_file.write_all(&payload_bytes)?;
        tmp_file.sync_all()?;
        drop(tmp_file);

        // 2. Atomic rename
        fs::rename(&tmp_path, &final_path)?;

        // 3. Parent directory fsync barrier for power-loss durability
        if let Ok(parent_dir) = File::open(dir) {
            let _ = parent_dir.sync_all();
        }

        Ok(())
    }

    pub fn load_snapshot(
        data_dir: impl AsRef<Path>,
    ) -> io::Result<Option<StateSnapshotData>> {
        let path = data_dir.as_ref().join("state.db");
        if !path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != STATE_DB_MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid state.db magic header"));
        }

        let mut version = [0u8; 1];
        file.read_exact(&mut version)?;
        if version[0] != STATE_DB_VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Unsupported state.db version: {}", version[0])));
        }

        let mut checksum_bytes = [0u8; 4];
        file.read_exact(&mut checksum_bytes)?;
        let expected_checksum = u32::from_le_bytes(checksum_bytes);

        let mut len_bytes = [0u8; 8];
        file.read_exact(&mut len_bytes)?;
        let payload_len = u64::from_le_bytes(len_bytes) as usize;

        let mut payload = vec![0u8; payload_len];
        file.read_exact(&mut payload)?;

        let computed_checksum = crc32(&payload);
        if computed_checksum != expected_checksum {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC32 checksum mismatch in state.db"));
        }

        let snapshot: StateSnapshotData = bincode::deserialize(&payload)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Deserialization error: {:?}", e)))?;

        Ok(Some(snapshot))
    }

    pub fn compact_wal(
        data_dir: impl AsRef<Path>,
        _retained_mutations: &[Mutation],
    ) -> io::Result<()> {
        let wal_path = data_dir.as_ref().join("wal.log");
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&wal_path)?;

        file.write_all(WAL_MAGIC)?;
        file.write_all(&[WAL_VERSION, 0, 0, 0])?;
        file.sync_all()?;

        // Parent dir barrier
        if let Ok(parent_dir) = File::open(data_dir.as_ref()) {
            let _ = parent_dir.sync_all();
        }

        Ok(())
    }
}
