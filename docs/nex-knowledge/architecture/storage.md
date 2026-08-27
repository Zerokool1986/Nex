# Storage & Persistence Architecture

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-core/src/storage/`, `NEX/00_CONSTITUTION/NEX-02_LOCAL_FIRST_STATE.md`, `NEX/00_CONSTITUTION/NEX-WAL-v1.md`  

---

## 1. On-Disk Directory Layout

`[IMPLEMENTATION OBSERVATION]`
For any active node data directory (e.g., `d:/Nex/nex_desktop_data/` or a temporary test directory), the physical disk structure comprises:

```text
node_data_dir/
├── .nex.lock         # Process ID lockfile preventing concurrent daemon instances
├── wal.log           # Append-only Write-Ahead Log journal (NEX/WAL/v1 framing)
├── state.db          # Two-Phase Checkpoint snapshot (Bincode + CRC-32)
├── state.db.tmp      # Staging file during snapshot creation
└── cas/              # Content-Addressed Storage binary chunk files (SHA-256 indexed)
```

---

## 2. Write-Ahead Log Engine (`WriteAheadLog`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/storage/wal.rs`:
- **Opening:** Reads or initializes 8-byte header (`NEXW` + `[1, 0, 0, 0]`).
- **Appending:** Formats `[len: 4B BE, type: 1B, payload: JSON/CBOR, crc: 4B BE]`. Flushes and issues `sync_data()` immediately.
- **Recovery & Auto-Truncation:** Reads records sequentially. If an EOF, length out-of-bounds, partial record, or CRC-32 checksum mismatch occurs:
  1. Recovery terminates at `last_valid_offset`.
  2. If file size exceeds `last_valid_offset`, it invokes `f.set_len(last_valid_offset)` and `f.sync_all()`, cleanly removing torn tail bytes.

---

## 3. Two-Phase Snapshot Engine (`StateDbEngine`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/storage/state_db.rs`:
```rust
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
```

### Snapshot & Compaction Protocol:
1. Serialize `StateSnapshotData` to memory via `bincode`.
2. Compute IEEE 802.3 CRC-32 checksum over serialized payload.
3. Write `state.db.tmp` header: Magic `NEXS` (4B) + Version `1` (1B) + CRC-32 (4B LE) + Length (8B LE) + Payload.
4. Execute `sync_all()` on `state.db.tmp`.
5. Atomically rename `state.db.tmp` $\to$ `state.db`.
6. Issue `sync_all()` on parent directory.
7. Truncate `wal.log` and write fresh 8-byte header `NEXW` + `[1, 0, 0, 0]`.

---

## 4. Content-Addressed Storage & FastCDC (`FastCdcChunker`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/storage/cdc.rs`:
- Chunks arbitrary large files using FastCDC Gear Matrix rolling hash table (`GEAR_TABLE: [u32; 256]`).
- Default chunk parameters:
  - Minimum Chunk: 16 KB (`DEFAULT_MIN_CHUNK_SIZE`)
  - Average Chunk: 64 KB (`DEFAULT_AVG_CHUNK_SIZE`)
  - Maximum Chunk: 256 KB (`DEFAULT_MAX_CHUNK_SIZE`)
- Sub-chunk masks: Dual mask (`mask_small` and `mask_large`) prevents chunk size skew.
- Each chunk boundary records `offset`, `length`, and `chunk_hash: SHA256(chunk_data)`.
- Reassembly integrity is strictly verified via `FastCdcChunker::verify_integrity`.
