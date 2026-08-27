# Synchronization & Anti-Entropy Architecture

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-core/src/sync/`, `NEX/00_CONSTITUTION/NEX-01_CONSTITUTIONAL_ARCHITECTURE.md`, `NEX/02_SYSTEM/NEX_SYNC_MODEL.md`  

---

## 1. The 5-Phase SMT Anti-Entropy Protocol

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/sync/anti_entropy.rs`, nodes reconcile divergent causal histories without central coordinators through a 5-phase session:

```text
Node A (Initiator)                               Node B (Responder)
        │                                                │
        ├─── 1. SyncAdvertise (Frontier, SMT Root) ─────▶│
        │                                                │
        │◀─── 2. SyncDeltaRequest (Missing Mutation IDs) ─│
        │                                                │
        ├─── 3. SyncStreamBatch (Sorted Mutations) ─────▶│
        │                                                │
        │◀─── 4. SyncBatchAck (Ingested Count, Window) ───│
        │                                                │
        ├─── 5. SyncComplete (Final State Commitment) ──▶│
        │                                                │
        │◀─── Verification (Roots Match == Converged) ───▶│
```

---

## 2. Session Protocol Messages

`[IMPLEMENTATION OBSERVATION]`
- **`SyncAdvertise`**: `session_id: [u8; 16]`, `current_epoch`, `current_lamport`, `latest_checkpoint_root: [u8; 32]`, `frontier_mutation_ids: Vec<MutationID>`, `known_mutation_count`.
- **`SyncDeltaRequest`**: `session_id`, `requested_mutations: Vec<MutationID>`, `max_batch_items: usize`.
- **`SyncStreamBatch`**: `session_id`, `batch_index: u32`, `total_batches: u32`, `mutations: Vec<Mutation>`.
- **`SyncBatchAck`**: `session_id`, `batch_index: u32`, `ingested_count: usize`, `remaining_window_credit: u32`.
- **`SyncComplete`**: `session_id`, `state_commitment: [u8; 32]`.

---

## 3. Causal Ancestor Traversal & Batch Sorting

`[IMPLEMENTATION OBSERVATION]`
In `AntiEntropyEngine::generate_batches_for_peer()`:
1. Reconstructs all known causal ancestors of the remote peer's frontier by backwards queue traversal in the local DAG.
2. Identifies all candidate mutations present locally that are outside the remote node's causal history.
3. Sorts candidate mutations deterministically:
   $$\text{order} = (\text{Epoch ASC}, \text{Lamport ASC}, \text{MutationID ASC})$$
4. Chunks mutations into sized batches (default 64 or 100 items).

---

## 4. Offline Outbox & Background Sync

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/sync/outbox.rs`:
- When offline, local mutations are committed to `wal.log` and queued in the durable `OfflineOutbox`.
- Background daemons periodically scan connected transport conduits and dispatch pending mutations upon link re-establishment.
- Deduplication prevents redundant transmissions of already-replicated mutations.
