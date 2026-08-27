# Changelog

All notable changes to the NEX Sovereign Platform will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **NEX Desktop UI Excellence Overhaul**
  - Tactile segmented Experience Slider (Simple | Standard | Advanced | Operator) in top chrome
  - Categorized semantic navigation rail (Spaces → Lenses → Mesh & Trust → System)
  - Sovereign Command Bar (`Ctrl+K` / `Cmd+K`) — Raycast-style spotlight launcher
  - Constellation Radar topology view with glassmorphic node cards and glowing spline links
  - Phosphor vector icon system across all UI surfaces
  - Drive file viewport with export, inspect, and share actions
  - Sovereign Node Control Center settings (master key backup, CAS GC, transport config)

- **NEX Design Intelligence Stack**
  - `nex-ui-ux-constitution` skill — Constitutional UX invariants and 10 Commandments enforcement
  - `nex-human-product-designer` skill — Product design reasoning, mental models, and usability analysis
  - `nex-visual-design-system` skill — Obsidian void design tokens, brand geometry, and Phosphor standards
  - Design Intelligence Loop matrix documentation

- **Repository Documentation**
  - Professional README with branded logo, architecture diagrams, and ecosystem overview
  - LICENSE (MIT), CONTRIBUTING.md, SECURITY.md, CHANGELOG.md
  - GitHub issue and pull request templates

## [0.1.0] — 2026-08-15

### Added

- **Constitutional Foundation**
  - NEX-00 through NEX-05 sovereignty specifications
  - NEX/WIRE/v1 48-byte fixed header wire specification (frozen)
  - NEX/WAL/v1 8-byte magic append-only journal specification (frozen)
  - 8-level authority hierarchy and sealed gate specifications

- **Core Substrate (`nex-core`)**
  - Causal state DAG with Lamport clock ordering
  - Sparse Merkle Tree (SMT) state commitments and accumulator
  - Write-Ahead Log with auto-truncation and crash recovery
  - Two-phase atomic checkpointing (state.db)
  - FastCDC content-addressed chunked storage with SHA-256
  - Ed25519 sovereign identity with capability token chain
  - Shamir social recovery over GF(256)
  - 5-phase anti-entropy synchronization engine
  - Offline durable outbox queue
  - TCP transport adapter with LAN discovery
  - Reticulum mesh radio native adapter
  - Universal Object Model (NexObject, ObjectType, Inode)
  - NexAppApi and 8 universal platform primitives
  - C ABI v1 and JNI bridge with DirectByteBuffer zero-copy
  - JSON-RPC 2.0 dispatcher for IPC
  - WASM fuel-metered compute sandbox (simulation)
  - Proof-of-Retrievability (PoR) HMAC challenge-response

- **Desktop Application (`nex-desktop`)**
  - Native egui/eframe desktop application with wgpu renderer
  - Home, Drive, Photos, Media, Maps, People, Network, Inspector, Settings surfaces
  - NEX brand identity and master geometric system
  - 84 desktop UI tests plus visual harness suite

- **Android Host (`android`)**
  - Kotlin application shell with Compose UI
  - NexKeystoreProvider with AndroidKeyStore TEE integration
  - NexSocketSyncService for LAN socket synchronization

- **Authoritative Knowledge Base (`docs/nex-knowledge`)**
  - 9 deep architecture documents (object model, canonical state, storage, sync, identity, transport, application boundary, platform realization, UX)
  - 6 constitutional analysis documents
  - 2 frozen contract specifications (WIRE-v1, WAL-v1)
  - Implementation status matrix with 10-level evidence ladder
  - Comprehensive test evidence mapping (108 suites, 648+ tests)
  - Known gaps and architectural seam analysis

- **Test Matrix**
  - 108 test suites across `nex-core`
  - 648+ individual test assertions
  - Chaos-verified (L6) coverage for DAG, SMT, WAL, checkpointing, capabilities, sync
  - Physical sync (L7) coverage for TCP transport
  - 20-step sovereign human journey end-to-end test
