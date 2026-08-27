# NEX-01: Constitutional Substrate & Mathematical Invariants

**Authority:** NEX Supreme Constitutional Law (Level 1)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-01_CONSTITUTIONAL_ARCHITECTURE.md`  
**Status:** FROZEN & IMMUTABLE  

---

## 1. Immutable Mathematical Principles

`[DIRECT SOURCE FACT]`
1. **Mathematical State DAG:** All state transitions are modeled as mutations in a Directed Acyclic Graph (DAG), deterministically ordered by `(Epoch ASC, LamportRank ASC, MutationID ASC, OperationIndex ASC)`.
2. **Sparse Merkle Tree (SMT):** State commitments are derived deterministically:
   $$\text{StateCommitment} = \text{SHA256}(\text{"NEX/STATE\_COMMITMENT/v1"} \,\|\, \text{SMT\_Root})$$
3. **Idempotency & Convergence:** Operations must converge identically across all nodes regardless of arrival order. Replays are mathematically idempotent.

---

## 2. Implementation Grounding in Substrate

`[IMPLEMENTATION OBSERVATION]`
- In `nex-core/src/model.rs`, mutations are defined by `MutationBody` containing `parents: Vec<MutationID>`, `lamport: u64`, `epoch: u64`, `is_resurrect: bool`, and `payload: CrdtPayload`.
- In `nex-core/src/sync/node.rs` (`VirtualNode::ingest_mutation`), mutations are causally validated:
  - Genesis mutations must have `lamport == 0` and `epoch == 0`.
  - Non-genesis mutations must have `lamport == max(parent_lamport) + 1`.
  - Resurrection mutations advance `epoch = max(parent_epoch) + 1`.
- In `nex-core/src/accumulator.rs`, the SMT algebra evaluates a 256-level binary tree with 3 discrete update outcomes:
  1. `Inserted(AccumulatorRoot)` — empty leaf slot updated.
  2. `NoOp(AccumulatorRoot)` — identical commitment already present.
  3. `Conflict` — slot contains a conflicting commitment (collision/forgery rejected).

---

## 3. Governance Scope

- **What NEX-01 Governs:**
  1. Causal DAG construction, Lamport clock mathematics, and epoch progression.
  2. Sparse Merkle Tree state commitment computation and inclusion/non-inclusion proofs.
  3. Mathematical idempotency and deterministic convergence invariants.
- **What NEX-01 Explicitly Does NOT Govern:**
  1. High-level domain entity schema types (e.g., `DriveInode`, `PhotoMedia`, which belong to the Universal Object Model layer).
  2. On-disk byte layouts and compaction intervals (governed by `NEX-02` and `NEX/WAL/v1`).
  3. Identity keys and cryptographic capability validation (governed by `NEX-03`).
