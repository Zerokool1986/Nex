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
