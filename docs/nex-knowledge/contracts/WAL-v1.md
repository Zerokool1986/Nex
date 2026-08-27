# NEX/WAL/v1: Write-Ahead Log Specification

**Authority:** NEX Frozen Persistence Contract (Level 2)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-WAL-v1.md`  
**Status:** STRICTLY FROZEN & IMMUTABLE  

---

## 1. Magic Header Layout (8 Bytes Exact)

`[DIRECT SOURCE FACT]`
- Every valid WAL file begins with an 8-byte ASCII header:
  ```text
  ASCII: "NEXWAL01"
  Hex:   0x4E 0x45 0x58 0x57 0x41 0x4C 0x30 0x31
  ```

---

## 2. Record Framing Layout

`[DIRECT SOURCE FACT]`
Each mutation record appended to the WAL contains:
1. `record_length: u32` (Big-Endian, 4 bytes)
2. `record_crc32: u32` (IEEE 802.3 CRC-32 checksum of the payload, 4 bytes)
3. `mutation_payload: [u8; record_length]` (Deterministic CBOR/binary encoded mutation)

---

## 3. Substrate Implementation Mechanics

`[IMPLEMENTATION OBSERVATION]`
- In `nex-core/src/storage/wal.rs`:
  - `WriteAheadLog::open()` verifies or initializes the 8-byte header on disk (`NEXW` + `[1, 0, 0, 0]`).
  - `WriteAheadLog::append_mutation()` prepends 4-byte length and 1-byte record type `RECORD_MUTATION` (`0x01`), writes the serialized mutation, and computes CRC-32 checksum over `(length || record_type || payload)`. It issues `file.sync_data()` immediately.
  - `WriteAheadLog::recover()` reads valid records sequentially. If a power cut or abrupt crash produced a torn record (partial length, truncated body, or CRC mismatch), recovery cleanly stops at the last valid record and truncates the file back to the valid commit offset.
- In `nex-core/src/storage/state_db.rs`:
  - Compaction writes an empty 8-byte header `NEXW` + `[1, 0, 0, 0]` to `wal.log` after `state.db` snapshotting succeeds.

---

## 4. Invariants

- **Durability Precedence:** An in-memory mutation or CRDT transition is NEVER valid until its `NEX/WAL/v1` frame has been successfully fsynced to disk.
- **Torn Write Immunity:** Incomplete records at the tail of the log are automatically detected and dropped without corrupting prior state.
