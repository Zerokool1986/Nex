use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::model::Mutation;

pub const WAL_MAGIC: &[u8; 4] = b"NEXW";
pub const WAL_VERSION: u8 = 1;

pub const RECORD_MUTATION: u8 = 1;
pub const RECORD_CHECKPOINT: u8 = 2;

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

pub struct WriteAheadLog {
    path: PathBuf,
    file: File,
}

impl WriteAheadLog {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let exists = path.exists();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        if !exists || file.metadata()?.len() == 0 {
            // Write 8-byte WAL Header: Magic (4B) + Version (1B) + Reserved (3B)
            file.write_all(WAL_MAGIC)?;
            file.write_all(&[WAL_VERSION, 0, 0, 0])?;
            file.flush()?;
        }

        file.seek(SeekFrom::End(0))?;

        Ok(Self { path, file })
    }

    pub fn append_mutation(&mut self, mutation: &Mutation) -> io::Result<()> {
        let payload = serde_json::to_vec(mutation)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let len = (payload.len() + 1) as u32;
        let mut record_data = Vec::with_capacity(5 + payload.len());
        record_data.extend_from_slice(&len.to_be_bytes());
        record_data.push(RECORD_MUTATION);
        record_data.extend_from_slice(&payload);

        let checksum = crc32(&record_data);

        self.file.write_all(&record_data)?;
        self.file.write_all(&checksum.to_be_bytes())?;
        self.file.flush()?;
        self.file.sync_data()?;

        Ok(())
    }

    pub fn recover(path: impl AsRef<Path>) -> io::Result<Vec<Mutation>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut file = File::open(path)?;
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() {
            return Ok(Vec::new()); // Empty / incomplete header
        }

        if &header[0..4] != WAL_MAGIC || header[4] != WAL_VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid WAL header magic or version"));
        }

        let mut mutations = Vec::new();
        let mut last_valid_offset = 8u64;

        loop {
            let mut len_bytes = [0u8; 4];
            if file.read_exact(&mut len_bytes).is_err() {
                break; // Clean EOF
            }
            let len = u32::from_be_bytes(len_bytes) as usize;
            if len == 0 || len > 16 * 1024 * 1024 {
                break; // Corrupted length boundary
            }

            let mut body_bytes = vec![0u8; len];
            if file.read_exact(&mut body_bytes).is_err() {
                break; // Partial record / abrupt crash cut-off
            }

            let mut crc_bytes = [0u8; 4];
            if file.read_exact(&mut crc_bytes).is_err() {
                break; // Partial CRC / crash cut-off
            }
            let expected_crc = u32::from_be_bytes(crc_bytes);

            let mut full_record = Vec::with_capacity(4 + len);
            full_record.extend_from_slice(&len_bytes);
            full_record.extend_from_slice(&body_bytes);

            if crc32(&full_record) != expected_crc {
                break; // Corrupted record from power loss -> stop recovery at last valid commit
            }

            let rec_type = body_bytes[0];
            let payload = &body_bytes[1..];

            if rec_type == RECORD_MUTATION {
                if let Ok(m) = serde_json::from_slice::<Mutation>(payload) {
                    mutations.push(m);
                    last_valid_offset += 4 + len as u64 + 4;
                } else {
                    break;
                }
            } else {
                last_valid_offset += 4 + len as u64 + 4;
            }
        }

        drop(file);

        // Auto-truncate torn tail bytes on disk back to last valid offset
        if let Ok(f) = OpenOptions::new().write(true).open(path) {
            if let Ok(meta) = f.metadata() {
                if meta.len() > last_valid_offset {
                    let _ = f.set_len(last_valid_offset);
                    let _ = f.sync_all();
                }
            }
        }

        Ok(mutations)
    }
}
