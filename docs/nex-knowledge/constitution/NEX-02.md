# NEX-02: Local-First State & Persistence

**Authority:** NEX Supreme Constitutional Law (Level 1)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-02_LOCAL_FIRST_STATE.md`  
**Status:** FROZEN & IMMUTABLE  

---

## 1. Constitutional Directives

`[DIRECT SOURCE FACT]`
1. **Append-Only Write-Ahead Log (WAL):**
   Every mutation is written and fsynced to `wal.log` before being reflected in memory or broadcast.
2. **Two-Phase Checkpointing:**
   Snapshots (`state.db`) are created via an atomic two-phase commit:
   1. Write snapshot to `state.db.tmp` and fsync.
   2. Atomic rename `state.db.tmp` $\to$ `state.db`.
   3. Truncate `wal.log` to the 8-byte header `NEXWAL01`.

---

## 2. Implementation Grounding in Substrate

`[IMPLEMENTATION OBSERVATION]`
- In `nex-core/src/storage/wal.rs`:
  - `WriteAheadLog::open()` writes the 8-byte magic header `NEXW` + `[1, 0, 0, 0]`.
  - `WriteAheadLog::append_mutation()` prepends 4-byte big-endian record length and 1-byte record type `RECORD_MUTATION`, followed by CBOR/JSON payload, and appends a 4-byte CRC-32 checksum. Calls `file.sync_data()` on every commit.
  - `WriteAheadLog::recover()` reads valid records up to any crash cutoff or CRC mismatch, drops invalid tail bytes, and truncates `wal.log` back to the last valid commit offset.
- In `nex-core/src/storage/state_db.rs`:
  - `StateDbEngine::save_snapshot()` writes `NEXS` (magic) + version `1` + CRC32 checksum + payload length + bincode snapshot, executes `sync_all()`, renames `state.db.tmp` to `state.db`, and executes a directory barrier `sync_all()`.
  - `StateDbEngine::compact_wal()` truncates `wal.log` and writes the 8-byte header.

---

## 3. Governance Scope

- **What NEX-02 Governs:**
  1. The "Disk First, Memory Second" durability invariant.
  2. The two-phase atomic snapshot sequence.
  3. Crash recovery semantics and WAL compaction rules.
- **What NEX-02 Explicitly Does NOT Govern:**
  1. Network transport framing (governed by `NEX-04` and `NEX/WIRE/v1`).
  2. CRDT state conflict resolution algorithms (governed by `NEX-01`).
  3. Chunking algorithms for large binary assets (governed by FastCDC in `src/storage/cdc.rs`).
