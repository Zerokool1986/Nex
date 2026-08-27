# Initial Architecture Archaeology Audit Report

**Date:** 2026-08-26  
**Auditor:** NEX Repository Archaeology & Documentation Agent  
**Mandate:** Authoritative, Evidence-Backed Repository Investigation & Knowledge Layer Construction  

---

## 1. Executive Summary

This audit establishes the baseline architectural ground truth of the NEX repository (`d:\Nex`). Through deep source inspection, symbol indexing, and test suite analysis, the repository was cataloged into its constitutional laws, frozen contracts, canonical Rust substrate, host platforms (Desktop and Android), and empirical test matrices.

---

## 2. Discovered Repository Architecture

The repository exhibits a clearly tiered architecture:

1. **Constitutional & Policy Root (`NEX/`, `.agents/rules/`):**
   - Contains the 6 constitutional documents (`NEX-00` .. `NEX-05`), frozen wire and persistence specs (`NEX-WIRE-v1`, `NEX-WAL-v1`), 4 sealed ADRs, UX constitution (`NEX-UX-01`), and the 8-level Authority Hierarchy.
2. **Canonical Core Engine (`nex-core/`):**
   - Pure Rust implementation of the sovereign substrate:
     - `src/model.rs`, `src/accumulator.rs`, `src/sync/node.rs`: Mathematical DAG, Lamport clocks, SMT 256-level tree.
     - `src/storage/`: Write-Ahead Log with auto-truncation (`wal.rs`), Two-Phase Checkpointing (`state_db.rs`), FastCDC chunker (`cdc.rs`).
     - `src/identity/`: Ed25519 ActorID derivation, CapabilityProof token verification, Shamir GF(256) social recovery.
     - `src/sync/`: 5-phase SMT anti-entropy sync engine (`anti_entropy.rs`), durable offline outbox (`outbox.rs`).
     - `src/transport/`: Pluggable transport adapters (`TcpTransportAdapter`, `ReticulumNativeAdapter`, `LanTcpTransportServer`).
     - `src/api/`: `NexAppApi` trait, `NexCoreRuntime`, `NexObjectStore`.
     - `src/ffi/` & `src/ipc/`: C ABI v1, JNI DirectByteBuffer bridge, JSON-RPC 2.0 dispatcher.
3. **Desktop Host Application (`nex-desktop/`):**
   - Native Rust GUI using `egui`/`eframe`. Implements Home Shell, Drive, Photos, People, Inspector, and the 4-step Experience Slider.
4. **Android Mobile Host Application (`android/`):**
   - Kotlin/JNI host application containing `NexClientApp.kt`, `NexKeystoreProvider.kt` (truthful TEE inspection), `NexSocketSyncService.kt` (LAN SMT socket sync), and `NexCameraManager.kt`.
5. **Authoritative Test Suites (`nex-core/tests/`):**
   - 108 test files covering all sealed gates from baseline conformance through Gate R72 (20-Step Sovereign Human Journey).
6. **Historical / Prototype Artifacts (`core/`, `*.py` in root):**
   - Early Python lab prototypes (`lab1b_core.py`, `engines.py`, `core.py`) and Go prototype files (`core/identity.go`, `core/protocol.go`). Subordinate to the active Rust substrate (`nex-core`).

---

## 3. Discrepancy & Seam Ledger

| Seam ID | Description | Source Discrepancy | Architectural Resolution / Status |
|---|---|---|---|
| **SEAM-01** | Wire Header Divergence | 48-byte `NEXW` in `NEX-WIRE-v1.md` vs 13-byte `NX` in `transport/types.rs`. | `[OPEN QUESTION]`: 13-byte header wraps transport segments; 48-byte header defines end-to-end AEAD session layer. |
| **SEAM-02** | Universal Object Model Placement | Universal Object Model is not explicitly defined in `NEX-01` or `NEX-05`. | `[FACT]`: Defined in `NEX/02_SYSTEM/NEX_DATA_MODEL.md` and `nex-core/src/object/types.rs`. Sits above the mathematical DAG/CRDT substrate. |
| **SEAM-03** | Gate P0 vs R71/R72 Test Matrix | `04_GATES/INDEX.md` records Gates P0-1..P0-7, while test suites use `r71_*` / `r72_*`. | `[INFERENCE]`: P0 gates represent post-R72 productization milestones realized by the R71/R72 product test suites and host applications. |

---

## 4. Verification & Validation Summary

- **Source Code Integrity:** No implementation files or frozen constitutional documents were modified.
- **Documentation Completeness:** All 23 planned knowledge layer documents have been generated in `docs/nex-knowledge/` with strict epistemic tagging.
- **Audit Conclusion:** The NEX repository represents an exceptionally disciplined, evidence-backed local-first sovereign platform codebase.
