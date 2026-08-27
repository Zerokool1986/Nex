# NEX Canonical Terminology

- **ActorID:** 32-byte cryptographic identifier derived from an Ed25519 public key.
- **NamespaceID:** 32-byte cryptographic domain separating distinct application or group enclaves.
- **ObjectID:** 32-byte deterministic identifier for an object in the Sparse Merkle Tree.
- **SMT (Sparse Merkle Tree):** 256-level cryptographic tree providing verifiable state roots and inclusion proofs.
- **WAL (Write-Ahead Log):** Append-only on-disk journal guaranteeing durability before in-memory state transition.
- **Capability Token:** Cryptographically signed authorization granting specific bitmask operations over a namespace/object.
- **Petname:** Locally assigned, human-readable name bound to an ActorID, resolved transitively via Web of Trust.
- **CAS (Content-Addressed Storage):** Storage engine where chunks are indexed by their SHA-256 hash.
- **PoR (Proof of Retrievability):** Cryptographic challenge-response verifying remote shard retention.
