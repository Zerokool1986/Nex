# NEX-01: Constitutional Substrate & Mathematical Invariants

## 1. Immutable Principles
1. **Mathematical State DAG:** All state transitions are modeled as mutations in a Directed Acyclic Graph (DAG), deterministically ordered by `(Epoch ASC, LamportRank ASC, MutationID ASC, OperationIndex ASC)`.
2. **Sparse Merkle Tree (SMT):** State commitments are derived deterministically: `StateCommitment = SHA256("NEX/STATE_COMMITMENT/v1" || SMT_Root)`.
3. **Idempotency & Convergence:** Operations must converge identically across all nodes regardless of arrival order. Replays are mathematically idempotent.
