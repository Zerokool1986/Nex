# NEX Capability & Delegation Model

Operations require an explicit `CapabilityProof` containing a `CapabilityToken`:
- `allowed_operations: u32` (Bitmask: `OP_READ`, `OP_WRITE`, `OP_DELEGATE`, `OP_ALL`)
- `valid_epochs: (u64, u64)`
- `delegation_depth: u32`
