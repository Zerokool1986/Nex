# NEX Master Architectural Map

**Status:** Authoritative Repository Landscape  
**Authority Level:** Level 1 $\to$ Level 7 Grounding  

---

## 1. System Topology Overview

```text
[ CLIENT / HOST LAYER ]
┌───────────────────────────────────────┐  ┌───────────────────────────────────────┐
│ NEX Desktop (egui / eframe / IPC)      │  │ NEX Android Client (Kotlin / JNI)      │
│ - Native OS Window & Event Loop        │  │ - Compose UI / Lifecycle Services      │
│ - IPC JSON-RPC Client / UDS Socket     │  │ - NexSocketSyncService / LAN Socket    │
│ - File Drag-and-Drop / Ingest          │  │ - AndroidKeyStore / TEE Hardware Cert  │
└───────────────────┬───────────────────┘  └───────────────────┬───────────────────┘
                    │                                           │
                    ▼                                           ▼
[ BOUNDARY / FFI LAYER ]
┌──────────────────────────────────────────────────────────────────────────────────┐
│ C ABI v1 (`nex-core/src/ffi/c_abi.rs`) & JNI Bridge (`src/ffi/jni_bridge.rs`)       │
│ - DirectByteBuffer Zero-Copy Passing                                               │
│ - HandleRegistry (`NexHandle`, `RuntimeInstance`)                                  │
│ - JSON-RPC 2.0 Dispatcher (`nex-core/src/ipc/rpc.rs`)                              │
└─────────────────────────────────────────┬────────────────────────────────────────┘
                                      │
                                      ▼
[ CANONICAL CORE SUBSTRATE (`nex-core`) ]
┌──────────────────────────────────────────────────────────────────────────────────┐
│ NEX Core Runtime (`NexCoreRuntime`, `NexNode`)                                     │
│                                                                                    │
│ ┌──────────────────────────┐ ┌──────────────────────────┐ ┌──────────────────────┐ │
│ │ Identity & Authorization │ │ Sovereign State Engine   │ │ Local Storage Engine │ │
│ │ - Ed25519 ActorID        │ │ - VirtualNode DAG        │ │ - Append-Only WAL    │ │
│ │ - CapabilityProof Chain  │ │ - SMT (Sparse Merkle)    │ │ - 2-Phase state.db   │ │
│ │ - Revocation Epoch Fence │ │ - CRDT LWW Register      │ │ - FastCDC CAS Chunks │ │
│ └──────────────────────────┘ └──────────────────────────┘ └──────────────────────┘ │
│                                                                                    │
│ ┌──────────────────────────┐ ┌──────────────────────────┐ ┌──────────────────────┐ │
│ │ Anti-Entropy Sync Engine │ │ Universal Object Store   │ │ Transport Dispatcher │ │
│ │ - 5-Phase SMT Protocol   │ │ - NexObject (Type/NS)    │ │ - TCP Socket Server  │ │
│ │ - Frontier Reconciliation│ │ - Inode & Media Metadata │ │ - Reticulum Native   │ │
│ │ - Outbox Durable Queue   │ │ - Synthetic Fallback     │ │ - QUIC / WebRTC / IPC│ │
│ └──────────────────────────┘ └──────────────────────────┘ └──────────────────────┘ │
└─────────────────────────────────────────┬────────────────────────────────────────┘
                                      │
                                      ▼
[ SOVEREIGN PRODUCT LENSES & SERVICES ]
┌──────────────────────────────────────────────────────────────────────────────────┐
│ Drive ── Photos ── Chat ── Communities ── Vault ── Backup ── Maps ── Web Gateway  │
│ (All applications consume the 8 Universal Platform Primitives from `NexAppApi`)   │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Layer Taxonomy & Repository Manifest

| Subsystem Layer | Primary Code Path | Key Symbols / Structs | Governing Specs / Gates |
|---|---|---|---|
| **Constitutional Specs** | `NEX/00_CONSTITUTION/` | `NEX-00` .. `NEX-05`, `NEX-WIRE-v1`, `NEX-WAL-v1` | Level 1 & 2 Frozen Law |
| **System Specifications** | `NEX/02_SYSTEM/` | `NEX_DATA_MODEL.md`, `NEX_SYNC_MODEL.md`, etc. | Level 3 & 4 Sealed Specs |
| **UX Constitution** | `NEX/05_UX/` | `NEX-UX-01-CONSTITUTION.md` (10 Commandments, Slider) | Level 4 UX Baseline |
| **Product Specifications** | `NEX/03_PRODUCTS/` | `NEX_DRIVE.md`, `NEX_PHOTOS.md`, `NEX_CHAT.md` | Level 4 Product Models |
| **Core Substrate** | `nex-core/src/` | `NexNode`, `VirtualNode`, `WriteAheadLog`, `SparseMerkleTree` | Level 6 Substrate Engine |
| **Identity & Caps** | `nex-core/src/identity/` | `ActorID`, `CapabilityProof`, `CapabilityToken`, `Shamir` | Gates R50, R61, R71-4 |
| **Storage & CAS** | `nex-core/src/storage/` | `WriteAheadLog`, `StateDbEngine`, `FastCdcChunker` | Gates R50-2, R51-2, R69 |
| **Anti-Entropy Sync** | `nex-core/src/sync/` | `AntiEntropyEngine`, `VirtualNode`, `OfflineOutbox` | Gates R50-4, R61, R71-5 |
| **Transport Layer** | `nex-core/src/transport/` | `TcpTransportAdapter`, `ReticulumNativeAdapter`, `LanTcpServer` | Gate R50-1, R70, R71-8 |
| **Application Boundary** | `nex-core/src/api/` | `NexAppApi`, `NexCoreRuntime`, `CoreRuntimeError` | Gate R50-5, R52, R58 |
| **FFI & JNI** | `nex-core/src/ffi/` | `c_abi.rs`, `jni_bridge.rs`, `HandleRegistry` | Gates R55, R56, R71-1 |
| **IPC & RPC** | `nex-core/src/ipc/` | `NexRpcDispatcher`, `JsonRpcRequest`, `JsonRpcResponse` | Gates R57, R71-3 |
| **Product ViewModels** | `nex-core/src/product/` | `HomeScreenViewModel`, `UniversalObjectInspector`, `Amy` | Gates R71-6, R72, P0-1 |
| **Desktop UI Shell** | `nex-desktop/` | `NexDesktopApp` (egui/eframe), `ui/` (Home, Drive, Photos) | Gate P0-2, P0-4 |
| **Android Host App** | `android/` | `NexClientApp`, `NexKeystoreProvider`, `NexSocketSyncService` | Gates R71-2, P0-3, P0-5 |
| **Authoritative Tests** | `nex-core/tests/` | 108 test suites (`r50_*` .. `r72_*`, `conformance_tests.rs`) | Level 7 Test Matrix |

---

## 3. Core Architectural Lifecycles

### Mutation Lifecycle (`VirtualNode::ingest_mutation`)
1. **Preimage Validation:** Asserts `mutation.id == SHA256(MutationBody)`.
2. **Local Duplicate Check:** Idempotent discard if `dag.contains_key(&id)`.
3. **Dependency Check:** If parents are missing in DAG, stores in `dependency_buffer`.
4. **Causal Admissibility:** Asserts parents are sorted and Lamport clock strictly satisfies `lamport == max(parent_lamport) + 1`.
5. **State Admission:** Evaluates deterministic CRDT LWW register; updates DAG and frontier.
6. **Cascade Resolution:** Recursively inspects and releases unblocked entries from `dependency_buffer`.

### Persistence Lifecycle (`NexNode::checkpoint_and_compact`)
1. Compute deterministic `CheckpointBody` (SMT State Root, Causal Root, Admission Root, Boundary).
2. Serialize snapshot payload and compute CRC-32.
3. Write `state.db.tmp` and execute OS `sync_all()`.
4. Atomically rename `state.db.tmp` $\to$ `state.db`.
5. Issue directory `sync_all()` barrier on parent folder.
6. Truncate `wal.log` to clean 8-byte header (`NEXWAL01`).
