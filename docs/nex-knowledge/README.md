# NEX Repository Authoritative Knowledge Layer

**Status:** Evidence-Backed Architectural Baseline  
**Authority Hierarchy:** Level 1 (NEX Constitution) $\to$ Level 8 (Experimental)  
**Target Audience:** Autonomous Agents & Independent Claude Instances utilizing GitHub MCP / Filesystem Inspection  

---

## 1. Purpose of this Knowledge Base

This directory (`docs/nex-knowledge/`) establishes the authoritative, evidence-backed knowledge layer for the **NEX Sovereign Platform**. It enables independent reasoning and auditability directly from repository artifacts, source code, frozen contracts, and empirical test matrices, without reliance on transient conversational memory or unverified summaries.

### Epistemic Tagging Standards
Every claim within this knowledge base is strictly partitioned using the following epistemology:
- `[DIRECT SOURCE FACT]`: Verbatim citations from constitutional markdown files, frozen specifications, or sealed ADRs.
- `[IMPLEMENTATION OBSERVATION]`: Verified behaviors, structs, enums, functions, and layout observed in `nex-core`, `nex-desktop`, and `android` code.
- `[TEST EVIDENCE]`: Specific test assertions, suite names, and empirical boundaries observed in `nex-core/tests/`.
- `[INFERENCE]`: Architectural conclusions derived logically from combinations of facts, explicitly labeled.
- `[OPEN QUESTION]`: Unresolved seams, implementation divergences, or undocumented areas recorded without speculative resolution.

---

## 2. Directory Navigation Map

```text
docs/nex-knowledge/
├── README.md                                  # You are here: Scope, hierarchy & epistemic taxonomy
├── NEX-MASTER-MAP.md                          # High-level architecture map and system component topology
│
├── constitution/                              # Constitutional Layer 1 Specifications
│   ├── NEX-00.md                              # Master Vision, Product Architecture & Universal Platform Rule
│   ├── NEX-01.md                              # Constitutional Substrate & Mathematical DAG Invariants
│   ├── NEX-02.md                              # Local-First State, WAL & Two-Phase Persistence
│   ├── NEX-03.md                              # Self-Sovereign Identity, ActorID Derivation & Web of Trust
│   ├── NEX-04.md                              # Transport Abstraction Layer (TAL) & Multi-Conduit Independence
│   └── NEX-05.md                              # Security & Adversarial Threat Boundaries
│
├── contracts/                                 # Frozen Layer 2 Wire & Persistence Specifications
│   ├── WIRE-v1.md                             # NEX/WIRE/v1 48-byte Fixed Header Wire Specification
│   └── WAL-v1.md                              # NEX/WAL/v1 8-byte Magic Append-Only Journal Specification
│
├── architecture/                              # Deep Subsystem Architecture & Mechanics
│   ├── object-model.md                        # Universal Object Model (NexObject, ObjectType, Inodes)
│   ├── canonical-state.md                     # State DAG, Lamport Ordering, CRDTs & SMT Commitments
│   ├── storage.md                             # Two-Phase Checkpointing, FastCDC CAS & Disk Layout
│   ├── synchronization.md                     # SMT Anti-Entropy, 5-Phase Session & Outbox Delivery
│   ├── identity-trust.md                      # Ed25519 Root, CapabilityProofs, Revocations & Shamir Recovery
│   ├── transport.md                           # Transport Adapters (TCP, Reticulum Mesh, WebRTC, IPC)
│   ├── application-boundary.md                # NexAppApi, IPC RPC Server & Universal Platform Primitives
│   ├── platform-realization.md                # Host Realizations (Desktop egui/eframe, Android JNI/Keystore)
│   └── ux.md                                  # UX Constitution, 10 Commandments, 3-Way Separation & Slider
│
├── evidence/                                  # Empirical Verification & Status Matrices
│   ├── implementation-status.md               # Subsystem implementation classification (L0 - L9)
│   ├── test-evidence.md                       # Comprehensive test suite mapping across 108 test files
│   └── known-gaps.md                          # Architectural seams, divergences and productization gaps
│
└── audits/                                    # Historical & Structural Audits
    └── initial-architecture-archaeology.md    # Full report from the initial repository archaeology baseline
```

---

## 3. The 8-Level Authority Precedence

When investigating or resolving any question regarding NEX design or implementation, strictly ascend the **Authority Hierarchy**:

1. **Level 1: NEX Constitution (`NEX-00` .. `NEX-05`)** — Inviolable sovereignty, local-first, capability security.
2. **Level 2: Frozen Wire & Persistence Contracts (`NEX/WIRE/v1`, `NEX/WAL/v1`)** — Immutable binary wire and on-disk journal formats.
3. **Level 3: Sealed Architectural Decision Records (ADRs)** — Ratified architectural paths; explicitly rejected anti-patterns.
4. **Level 4: Sealed Gate Specifications (`R50` .. `R72`, `P0-1` .. `P0-7`)** — Mathematical formulas, state machines, subsystem interfaces.
5. **Level 5: Binding Contract Suites & FFI Definitions** — C ABI v1 signatures, JNI direct buffer layouts.
6. **Level 6: Canonical Rust Substrate Implementation** — `crates/nex-core` active source code and engines.
7. **Level 7: Authoritative Test Matrix (108 Suites / 648 Tests)** — Empirical test assertions and stress harnesses.
8. **Level 8: Experimental / Proposed Realization Work** — Draft specifications, feature branches, exploratory UI.
