# NEX Data Model & CRDT Semantics

All state in NEX is structured as `NexObject` entries within 32-byte namespaces:
- `object_id: [u8; 32]` (Deterministic hash of namespace, author, lamport, payload)
- `object_type: ObjectType`
- `metadata: BTreeMap<String, String>`
- `payload_bytes: Vec<u8>`
- `tombstoned: bool`
