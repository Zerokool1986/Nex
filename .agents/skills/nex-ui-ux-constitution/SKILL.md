---
name: nex-ui-ux-constitution
description: Authoritative NEX UI/UX Constitution, architectural invariants, and non-negotiable interaction rules.
---

# NEX UI/UX Constitution Skill

## Purpose & Authority
This skill establishes the **authoritative behavioral and architectural constraints** for all NEX user interfaces, screens, widgets, and interactions. It encodes and enforces:
- NEX Constitution (Levels 1-8) & NEX-00..05
- NEX-UX-01 (Authoritative UX Constitution)
- Frozen Wire & WAL Contracts (NEX/WIRE/v1, NEX/WAL/v1, C ABI v1)
- Sealed Gate Specifications (R50 through R73)
- Human Product Baseline v1 (referenced in DOCS)

It answers the foundational question:
> **"What is NEX allowed to feel like, and how must it behave?"**

---

## 1. Non-Negotiable Invariants

### 1.1 Truthful State Reporting (No Deceptive Optimism)
- **Forbidden:** Never show "Synced", "Safe in Cloud", or "Uploaded" when bytes only exist locally on the current device.
- **Required:**
  - When local only: Display **"Saved locally • Waiting for paired devices"** (Amber).
  - When replicated across mesh: Display **"Safe on 3 devices • Verified by DAG"** (Green).
  - When offline: Display **"Working offline • Will sync via LAN/Relay when connected"** (Slate).

### 1.2 Spaces x Lenses Orthogonality
- **Spaces (Boundary of Access & Trust):**
  - `Personal` (Root sovereign space; private to the user).
  - `Family` (Shared family circle; collective ownership).
  - `Community / Spaces` (Explicitly shared sovereign scopes).
- **Lenses (Projections of the Same Single Object DAG):**
  - `Home` (Recent activity, status, quick actions).
  - `Photos` (Visual media projection).
  - `Drive` (Filesystem & hierarchical document projection).
  - `Media` (Audio, video, streaming stream representations).
  - `Maps` (Spatial / geotag projection).
  - `People` (Identities, trust tiers, capability grants).
  - `Devices` (Physical hardware nodes, local mesh links).
  - `Network` (Sovereign topology and causal relationship graph).
  - `Settings` (Node configuration, key management, recovery).

### 1.3 Universal Object Grammar
- An object has a single canonical `ObjectID` (32-byte BLAKE3).
- Selecting an object in *any* lens and navigating to *any other* lens MUST preserve the selected `ObjectID` without drift.
- There are **no shadow databases, duplicate file records, or conflicting secondary tables**.

### 1.4 Universal Inspector Contract
The right-hand drawer is the single source of truth for any inspected entity (Object, Space, Person, Device):
1. **Identity & Provenance:** Title, Object ID hex, Schema Version, Creator Actor ID, Lamport timestamp.
2. **Capabilities & Trust:** Explicit permission tokens, delegation depth, and revocation status.
3. **Physical Residency:** Exact bytes stored on this node vs. remote paired hosts.
4. **Diagnostic Accordion (Governed by Experience Slider):**
   - *Simple:* "Safe on your PC and Living Room Node."
   - *Standard:* Replica breakdown, transport status, storage consumption.
   - *Advanced:* SMT root hash, Lamport epoch, capability token hex.
   - *Operator:* Raw WAL sequence, wire frame headers, cryptographic signatures.

### 1.5 Experience Slider (Progressive Disclosure)
- **Simple:* Calm, jargon-free summary (*"my photo is safe."*).
- **Standard:* Replicas, trusted people, and storage quotas.
- **Advanced:* Cryptographic proofs, transport protocols, and SMT roots.
- **Operator:* Raw low-level substrate logs and state machines.
- **Invariant:** Switching the slider is *strictly presentation-only (read-only)*. It must never mutate DAG state or capability tokens.

### 1.6 Proximity SAS Verification Ceremony
- Pairing and high-trust capabilities require a 4-word Short Authentication String (SAS) out-of-band confirmation (e.g. `RIVER - SUMMIT - FALCON - HARBOR`) or QR exchange.
- Zero silent escalation or unverified peer trust.
