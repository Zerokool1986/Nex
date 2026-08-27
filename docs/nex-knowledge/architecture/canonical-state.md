# Canonical State & Mathematical DAG Architecture

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-core/src/model.rs`, `nex-core/src/sync/node.rs`, `nex-core/src/accumulator.rs`, `NEX/00_CONSTITUTION/NEX-01_CONSTITUTIONAL_ARCHITECTURE.md`  

---

## 1. Causal DAG Data Structures

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/model.rs`:
```rust
pub struct MutationBody {
    pub author: Identity,
    pub parents: Vec<MutationID>,
    pub lamport: u64,
    pub epoch: u64,
    pub is_resurrect: bool,
    pub payload: CrdtPayload,
}

pub struct Mutation {
    pub id: MutationID,
    pub body: MutationBody,
}
```

### CRDT Payloads
```rust
pub enum CrdtPayload {
    AddLWW { id: [u8; 32], value: Vec<u8> },
    RemoveLWW { id: [u8; 32] },
    Tombstone { id: [u8; 32] },
}
```

---

## 2. Six-Phase Ingress Mutation Lifecycle

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/sync/node.rs` (`VirtualNode::ingest_mutation`), mutations pass through six discrete verification stages:

```text
Incoming Mutation
       │
       ▼
[ 1. Cryptographic Preimage Check ]  ──(m.id != SHA256(body))──▶ Return Invalid("Forged MutationID")
       │
       ▼
[ 2. Duplicate Lookup ]             ──(dag.contains(m.id))─────▶ Return Duplicate(m.id)
       │
       ▼
[ 3. Causal Dependency Check ]      ──(parents missing)────────▶ Store in DependencyBuffer
       │                                                         Return DependencyGap
       ▼
[ 4. Causal Admissibility Check ]   ──(parents unsorted)───────▶ Return Rejected
       │                            ──(lamport != max(p)+1)────▶ Return Rejected
       ▼
[ 5. CRDT State & DAG Admission ]   ──(Deterministic LWW)──────▶ Update crdt_state, dag, frontier
       │
       ▼
[ 6. Cascade Resolution ]           ──(Scan unblocked items)───▶ Recursively admit DependencyBuffer
```

---

## 3. Deterministic Ordering & Tie-Breaking

`[DIRECT SOURCE FACT]` & `[IMPLEMENTATION OBSERVATION]`
When competing mutations modify the same key, conflict resolution is strictly deterministic:
1. **Epoch (ASC):** Higher epoch strictly supersedes lower epoch.
2. **Lamport Rank (ASC):** If epochs match, higher Lamport rank wins.
3. **MutationID (ASC):** If both epoch and Lamport match, lexicographical comparison `mutation.id > existing.mutation_id` breaks ties.

---

## 4. Sparse Merkle Tree (SMT) State Commitments

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/accumulator.rs`:
- Tree Depth: Fixed at 256 levels.
- Leaf Key: `sha256_smt_key(mutation_id) = SHA256("NEX/SMT_KEY/v1" || mutation_id)`.
- Leaf Hash: `sha256_smt_leaf(commitment) = SHA256("NEX/SMT_LEAF/v1" || commitment)`.
- Node Hash: `sha256_smt_node(left, right) = SHA256("NEX/SMT_NODE/v1" || left || right)`.
- Tree Root: Derived by hashing all sorted entries under domain prefix `"NEX/SMT_TREE_ROOT/v1"`.
- Proof Verification: `verify_smt_inclusion(root, mutation_id, commitment, proof)` verifies a 256-hash sibling path from leaf to root.
