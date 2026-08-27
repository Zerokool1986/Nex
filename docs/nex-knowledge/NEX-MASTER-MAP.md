# NEX Master Architectural Map

**Status:** Authoritative Repository Landscape  
**Authority Level:** Level 1 $\to$ Level 7 Grounding  

---

## 1. System Topology Overview

```text
[ CLIENT / HOST LAYER ]
┌────────────────────────────────────────┐  ┌────────────────────────────────────────┐
│ NEX Desktop (egui / eframe / IPC)      │  │ NEX Android Client (Kotlin / JNI)      │
│ - Native OS Window & Event Loop        │  │ - Compose UI / Lifecycle Services      │
│ - IPC JSON-RPC Client / UDS Socket     │  │ - NexSocketSyncService / LAN Socket    │
│ - File Drag-and-Drop / Ingest          │  │ - AndroidKeyStore / TEE Hardware Cert  │
└───────────────────┬────────────────────┘  └───────────────────┬────────────────────┘
                    │                                           │
                    ▼                                           ▼
[ BOUNDARY / FFI LAYER ]
┌────────────────────────────────────────────────────────────────────────────────────┐
│ C ABI v1 (`nex-core/src/ffi/c_abi.rs`) & JNI Bridge (`src/ffi/jni_bridge.rs`)       │
│ - DirectByteBuffer Zero-Copy Passing                                               │
│ - HandleRegistry (`NexHandle`, `RuntimeInstance`)                                  │
│ - JSON-RPC 2.0 Dispatcher (`nex-core/src/ipc/rpc.rs`)                              │
└─────────────────────────────────────┬──────────────────────────────────────────────┘
                                      │
                                      ▼
[ CANONICAL CORE SUBSTRATE (`nex-core`) ]
┌────────────────────────────────────────────────────────────────────────────────────┐
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
└─────────────────────────────────────┬──────────────────────────────────────────────┘
                                      │
                                      ▼
[ SOVEREIGN PRODUCT LENSES & SERVICES ]
┌────────────────────────────────────────────────────────────────────────────────────┐
│ Drive ── Photos ── Chat ── Communities ── Vault ── Backup ── Maps ── Web Gateway  │
│ (All applications consume the 8 Universal Platform Primitives from `NexAppApi`)   │
└────────────────────────────────────────────────────────────────────────────────────┘
```
