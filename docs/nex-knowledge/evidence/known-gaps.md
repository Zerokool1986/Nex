# Known Architectural Gaps, Seams & Contradictions

**Status:** Authoritative Repository Archaeology Ledger  
**Source Baseline:** Repository Audit, `NEX/06_PRODUCT/NEX-POST-R72-GAP-MAP.md`, `NEX/04_GATES/INDEX.md`  

---

## 1. Architectural Seams & Divergences Discovered

### Seam 1: Transport Frame Header vs Frozen Wire v1 Header
- `[IMPLEMENTATION OBSERVATION]`: In `nex-core/src/transport/types.rs`, transport adapters use a **13-byte link frame** (`NX` magic 2B + transport tag 2B + flags 1B + length 4B + CRC32 4B).
- `[DIRECT SOURCE FACT]`: In `NEX/00_CONSTITUTION/NEX-WIRE-v1.md`, the frozen wire protocol defines a **48-byte fixed header** (`NEXW` magic 4B + version 2B + msg_type 2B + flags 2B + len 4B + nonce 8B + sender 16B + checksum 16B).
- `[INFERENCE]`: The 13-byte frame operates as an inner transport segment wrapper, while the 48-byte `NEXW` header represents the end-to-end AEAD cryptographic session envelope. However, the substrate code currently lacks an explicit adapter bridging the 13-byte link frame directly into the 48-byte AEAD session container in all test paths.

### Seam 2: P0 Gate Record in Documentation vs Codebase Test Naming
- `[DIRECT SOURCE FACT]`: `NEX/04_GATES/INDEX.md` lists Gates `P0-1` through `P0-7` as "SEALED (6/6)" with a total master test count of 648 tests.
- `[IMPLEMENTATION OBSERVATION]`: `NEX-POST-R72-GAP-MAP.md` lists the master baseline as "606 / 606 passing tests across 101 suites", while physical test files in `nex-core/tests/` are named under the `r71_*` and `r72_*` prefixes (e.g., `r72_4_twenty_step_sovereign_human_journey_tests.rs`).
- `[INFERENCE]`: P0-1 through P0-7 represent the post-R72 productization realization phases that map onto the R71/R72 product test suites and host applications (`nex-desktop`, `android`).

### Seam 3: Universal Object Model vs Raw CRDT Ingestion
- `[IMPLEMENTATION OBSERVATION]`: `VirtualNode` (the mathematical DAG substrate) ingests raw byte payloads under `CrdtPayload::AddLWW` and `CrdtPayload::Tombstone`.
- When an object mutation arrives via anti-entropy sync for an object not already present in `NexObjectStore`, `AntiEntropyEngine::ingest_batch()` generates a placeholder object with `ObjectType::Synthetic(1)` and empty metadata.
- `[OPEN QUESTION]`: Full synchronization of domain-specific typed metadata across nodes relies on higher-level object payload replication rather than direct CRDT field synchronization at the substrate DAG layer.

---

## 2. Unresolved / Incomplete Productization Areas

| Capability | Current State | Missing Realization Gap |
|---|---|---|
| **Android UI Surface** | Headless JNI + Service wrappers | Native Jetpack Compose UI screens (Home, Spaces, Photos grid). |
| **Android Camera Bridge** | Mock/Lifecycle structure (`NexCameraManager.kt`) | Real CameraX direct pixel piping into `CasChunkStore` on physical device. |
| **OS Hardware KeyStore** | Software Dalek fallback / Truthful inspection | Native hardware TEE/StrongBox key generation on physical Android hardware. |
| **Desktop Background Daemon** | Foreground `egui` app / CLI daemon | Windows Service / Linux systemd daemon auto-launching on boot. |
| **WASM Compute Mesh** | Fuel-metered simulation engine | Full multi-node sandboxed wasmtime runtime cluster in production. |
