# NEX-UX-OFFLINE-PATTERNS: Truthful Sync States, Degraded Mesh Modes & Calm Recovery

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Research Document  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. The Offline-First Constitutional Axiom

In cloud-dependent systems, the offline state is treated as an exceptional error: features disable themselves, gray out, or display intrusive alert banners ("You are offline - please reconnect").

**NEX UX Doctrine (Constitutional Invariant):**
> *In NEX, offline is the baseline normal state. Network connectivity is an opportunistic enhancement for replication, never a prerequisite for local existence or full user capability.*

Every action—creating a document, capturing a photo, assigning a permission, editing metadata, organizing albums—completes instantly in local memory and is journaled immediately to the append-only Write-Ahead Log (`NEX/WAL/v1`).

---

## 2. Truthful Synchronization Badges (The "Zero False Synced" Guarantee)

Traditional software lies to users:
- Google Drive displays a green checkmark the moment bytes hit Google's upload load balancer, even if the recipient device has not received the file.
- Cloud apps report "All changes saved" when they have only been placed in an ephemeral browser cache.

### The NEX 4-State Truth Matrix `[NEX-specific]`

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                             TRUTHFUL SYNC MATRIX                                 │
├───────────────────┬────────────────────────────────┬─────────────────────────────┤
│ State             │ UI Visual Indicator            │ Mathematical / Substrate    │
├───────────────────┼────────────────────────────────┼─────────────────────────────┤
│ 1. Local Only     │ 🟡 Amber Circle Dot            │ Object in local WAL/CAS;    │
│                   │ "Saved locally. Awaiting peer."│ zero remote ACK received    │
│ 2. Replicating    │ 🔵 Pulsing Blue Dot            │ SMT batch in flight over    │
│                   │ "Syncing with Desktop (45%)"   │ TAL socket or BLE stream    │
│ 3. Verified Sync  │ 🟢 Solid Green Dot             │ Peer returned ACK with      │
│                   │ "Safe on 3 Replicas"           │ identical SMT root hash     │
│ 4. Offline Outbox │ ⚪ Calm White Outline Dot       │ Queued in persistent Outbox;│
│                   │ "Offline. Will sync on connect"│ retry backoff active        │
└───────────────────┴────────────────────────────────┴─────────────────────────────┘
```

> [!IMPORTANT]
> **Constitutional Rule:** The UI must NEVER display a green "Synced" state unless the anti-entropy exchange has completed and the remote peer has acknowledged receipt of the exact CAS root hash.

---

## 3. Degraded Connectivity & Multi-Transport Mesh UX

When internet connectivity drops, NEX continues operating across local transports:
1. **Local Wi-Fi / LAN Direct:** Automatic high-speed CAS chunk synchronization between home devices.
2. **Wi-Fi Direct / Ad-Hoc P2P:** Phone-to-laptop synchronization in remote outdoor locations without routers.
3. **Bluetooth Low Energy (BLE) / Mesh:** Text chat and metadata frontier exchange when Wi-Fi is disabled.
4. **Sneakernet / Airgap (USB Drive / SD Card):** Export encrypted WAL batch to physical media and import on another node.

### How Degraded Transport is Communicated Without Inducing Anxiety `[Inferred]`

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ 🌐 MESH STATUS: LOCAL AD-HOC (NO INTERNET REQUIRED)                    │
│                                                                         │
│ 📱 Pixel 9 Pro ◀───(High Speed LAN: 120 MB/s)───▶ 💻 Studio Desktop    │
│                                                            │            │
│                                              (Direct Wi-Fi)│            │
│                                                            ▼            │
│                                              🏡 Living Room Node        │
│                                                                         │
│ ℹ️ Your devices are syncing directly over your home network.            │
│    All your data is completely up to date between your devices.         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Conflict Resolution UX: Deterministic & Human-Guided

NEX uses causal state DAGs and Lamport timestamps. In 99% of cases, mutations merge deterministically:
- **Disjoint Object Mutations (e.g. caption edited on Phone, tags added on Laptop):** Automatic deterministic union merge.
- **Concurrent Scalar Collisions (e.g. conflicting titles edited offline simultaneously):**
  - Substrate picks the winning mutation deterministically using highest Lamport timestamp + lexicographical ActorID hash tie-break.
  - The loser mutation is preserved in the object's provenance history as a branch, never silently erased.

### The Calm Conflict Resolution Sheet `[Experimental]`

When a user opens an object with concurrent offline branches, NEX presents a non-destructive choice:

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ 🔀 TWO EDITS WERE MADE WHILE OFFLINE                                    │
├─────────────────────────────────────────────────────────────────────────┤
│ Both versions are preserved. Which title would you like to keep?        │
│                                                                         │
│ (•) "Lake Tahoe Family Sunset"                                          │
│     Edited on 📱 Pixel 9 Pro (Aug 26, 14:32)                            │
│                                                                         │
│ ( ) "Tahoe Trip - Day 3"                                                │
│     Edited on 💻 Studio Desktop (Aug 26, 14:30)                         │
│                                                                         │
│ [ Keep Both as Separate Versions ]                  [ ✅ Apply Choice ] │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Summary of Offline UX Invariants

1. **Zero Blocked Operations `[NEX-specific]`:** All UI inputs and object mutations persist locally immediately.
2. **Truthful Messaging `[Observed]`:** Status pills never simulate sync completion.
3. **Calm Degradation `[Inferred]`:** Switching between Wi-Fi, BLE, and offline mode happens silently without interrupting active human tasks.
