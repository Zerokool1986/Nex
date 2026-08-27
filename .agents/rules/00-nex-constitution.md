---
description: Always-on NEX Constitutional Guidance & Core Behavioral Invariants
always_apply: true
---

# NEX Constitutional Framework & Agent Directives

You are working on **NEX**, a sovereign, local-first, decentralized personal computing, storage, communication, and resource mesh platform.

## 1. Core Identity & Axioms

- **Sovereignty First:** The user owns their identity, keys, data, and compute. No external authority can revoke the existence of a user's cryptographic ActorID; however, local sovereign domains retain the absolute right to revoke authorization to their resources.
- **Local-First & Data Ownership:** NEX has no mandatory remote plaintext authority. User-controlled local storage is authoritative, while data-at-rest protection (cleartext, full-disk encryption, zero-knowledge Vault ciphertext, OS hardware keystore wrapping) is governed by namespace policy and host device security capabilities.
- **Capability-Based Security:** Ambient authority is forbidden. All operations on objects, namespaces, storage, or compute must be authorized by explicit cryptographic capability tokens (`CapabilityProof`).
- **Cryptographic Trust & Active Services:** The network contains active distributed services (DHT, WebRTC, TAL, credit accounting, storage healing, compute scheduling), but NO network intermediary is authoritative over user identity, user data, or application state.

## 2. Inviolable Operational Directives

1. **Never Violate the Authority Hierarchy:**
   ```text
   1. NEX Constitution (NEX-00..05)
   2. Frozen Binary Wire & WAL Specifications (NEX/WIRE/v1, NEX/WAL/v1)
   3. Sealed Architectural Decision Records (ADRs)
   4. Sealed Gate Specifications (R50..R67)
   5. Binding Contracts (C ABI, JNI, FFI)
   6. Current Rust Substrate Implementation
   7. Authoritative Test Matrix
   8. Experimental / Proposed Enhancements
   ```
2. **Never Infer Production Readiness from Isolated Test Passes:**
   A passing test suite proves only conformance to a defined gate scope. It does NOT prove security against all adversaries, large-scale performance, or ecosystem interoperability.
3. **The Implementation Serves the Vision, Not the Inverse:**
   Never shrink the definition of NEX to fit what has already been programmed.
4. **Never Introduce Centralized Dependencies:**
   Do not add mandatory cloud coordinators, DNS lookups, blockchain consensus mechanisms, or central relay servers.
5. **Never Break Transport Independence:**
   NEX core is transport-agnostic. Reticulum, WebRTC, TCP/IP, BLE, and Unix Domain Sockets are modular transport plugins under the Transport Abstraction Layer.
6. **Never Compromise Usability for Ideological Purity:**
   NEX is designed for everyday human beings.
