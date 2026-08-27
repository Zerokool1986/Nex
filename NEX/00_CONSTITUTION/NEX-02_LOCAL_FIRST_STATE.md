# NEX-02: Local-First State & Persistence

## 1. Append-Only Write-Ahead Log (WAL)
Every mutation is written and fsynced to `wal.log` before being reflected in memory or broadcast. 

## 2. Two-Phase Checkpointing
Snapshots (`state.db`) are created via an atomic two-phase commit:
1. Write snapshot to `state.db.tmp` and fsync.
2. Atomic rename `state.db.tmp` -> `state.db`.
3. Truncate `wal.log` to the 8-byte header `NEXWAL01`.

## 3. Conflict Resolution & Materialized State Invariants
1. **Deterministic Last-Write-Wins (LWW) Ordering Rule:**
   When competing mutations modify the same object or register, precedence is strictly evaluated by the deterministic tuple:
   `(Epoch ASC, LamportRank ASC, MutationID ASC)`.
2. **Unified Materialized State Invariant:**
   All user-facing and application-facing state representations (including `object_store`, ViewModels, and secondary indices) MUST strictly reflect the winner determined by the causal LWW decision. Materialized views must never be updated independently of the causal ordering decision, regardless of network transport arrival order, sync batch boundaries, or full-object replication mechanisms.
3. **Losing Write Preservation:**
   Losing mutations are permanently retained in the immutable causal DAG and `wal.log` for cryptographic auditability and provenance, but are not exposed as active state in the user interface.
