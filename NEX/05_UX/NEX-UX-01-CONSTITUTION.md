# NEX-UX-01: NEX User Experience Constitution & Master Architectural Doctrine

**Status:** **AUTHORITATIVE CONSTITUTIONAL UX BASELINE (SEALED)**  
**Version:** 1.0.0  
**Authority:** Chris / NEX Architecture  
**Scope:** Universal across all NEX Surfaces (NEX Home, Drive, Photos, Backup, Vault, Media, Chat, Communities, Maps, Web)  
**Constitutional Foundation:** `NEX-00..05`, `NEX/WIRE/v1`, `NEX/WAL/v1`, C ABI v1 (**FROZEN & IMMUTABLE**)  

---

## 1. The Fundamental NEX UX Premise

> **NEX is one coherent personal environment, not a collection of unrelated applications.**

NEX does not present itself as seven competing cloud apps bundled together. NEX is the user's sovereign digital world. 

Applications are not siloed destinations; they are **specialized lenses** viewing the user's unified data, people, and hardware.

---

## 2. The 10 Commandments of NEX User Experience

```text
+====================================================================================================+
|                              THE 10 COMMANDMENTS OF NEX UX                                         |
+====================================================================================================+
| 1. ABSOLUTE CLARITY         | The user must immediately understand what their system is doing,     |
|                             | where their data lives, and who has access to it.                    |
| 2. USER SOVEREIGNTY        | The user has final, inviolable authority over their data, devices,   |
|                             | identities, and network connectivity.                                |
| 3. PREDICTABILITY           | Actions have deterministic, clear consequences. No magical background|
|                             | mutations or unexpected ambient side-effects.                        |
| 4. CONTEXT OVER ISOLATION   | Every object, person, and device is connected to its surrounding     |
|                             | relationships (Space, Trust, Capabilities, Replicas, History).       |
| 5. PROGRESSIVE DISCLOSURE   | Hide technical complexity until the user asks for it. Never simplify |
|                             | by removing control; simplify by staging disclosure.                 |
| 6. LOCAL-FIRST BY DEFAULT   | All features work completely offline. Network connectivity is an     |
|                             | enhancement for synchronization, not a prerequisite for existence.   |
| 7. EXPLICIT TRUST & CAPS    | Permissions are explicit, bounded, and human-readable. Ambient       |
|                             | authority is strictly prohibited.                                    |
| 8. CONSISTENT UX GRAMMAR    | Every surface uses the identical conceptual vocabulary (Person,      |
|                             | Identity, Device, Space, Object, Permission, Trust, Sync, Activity).  |
| 9. UNIVERSAL ACCESSIBILITY  | Fluid navigation, adaptive contrast, screen-reader semantics, and    |
|                             | platform-native ergonomics across Mobile and Desktop.                 |
| 10. EXPERT POWER ACCESSIBLE | Power users can inspect raw SMT trees, WAL frames, and transport     |
|                             | routing without forcing ordinary users to become engineers.          |
+====================================================================================================+
```

---

## 3. The 3-Way Separation Rule (Non-Negotiable UX Invariant)

```text
+====================================================================================================+
|                                  THE 3-WAY SEPARATION RULE                                         |
+====================================================================================================+
| 1. WHO ARE YOU?            | IDENTITY                                                              |
|                            | Permanent Root ActorID, Ed25519 Keys, Hardware DeviceCertificate.     |
+----------------------------+-----------------------------------------------------------------------+
| 2. WHAT CAN YOU DO?        | PERMISSIONS & AUTHORIZATION ROLES                                     |
|                            | Attenuated CapabilityProof Tokens (Owner, Admin, Member, Guest, Child)|
+----------------------------+-----------------------------------------------------------------------+
| 3. WHAT DO YOU WANT TO SEE?| INTERFACE COMPLEXITY & UX EXPERIENCE LEVEL                            |
|                            | Presentation Filter (Simple, Standard, Advanced, Expert).             |
+====================================================================================================+
```

> [!IMPORTANT]
> **CONSTITUTIONAL UX INVARIANT:**
> **Never conflate authorization with interface complexity.**
> Experience levels control visual presentation and progressive disclosure only; they must never silently grant, modify, or remove cryptographic authority.

---

## 4. The Interface Complexity Slider (`Settings → Experience`)

Instead of locking users into permanent account categories, NEX provides a dynamic **Interface Complexity Slider**:

```text
┌─────────────────────────────────────────────────────────────┐
│ ⚙️ Settings → Experience                                    │
│ ─────────────────────────────────────────────────────────── │
│ 🎚️ INTERFACE COMPLEXITY LEVEL                               │
│                                                             │
│ ( ) 🟢 SIMPLE                                               │
│     Calm, minimal view. NEX handles routine sync & storage  │
│     decisions automatically.                                │
│                                                             │
│ (•) 🔵 STANDARD (Recommended)                               │
│     Balanced view for daily life. Exposes Spaces, sharing   │
│     controls, device status, and storage quotas.            │
│                                                             │
│ ( ) 🟡 ADVANCED                                             │
│     Exposes detailed synchronization queues, offline outbox,│
│     mesh transport preferences, and granular capabilities.  │
│                                                             │
│ ( ) 🟣 EXPERT                                               │
│     Full diagnostic control: SMT Merkle proofs, WAL frames, │
│     erasure coding matrices, fuel metering, and raw logs.   │
│                                                             │
│ ℹ️ Changing this level does not affect your permissions,     │
│    security, or data safety. It only controls how much      │
│    information NEX displays by default.                     │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. Authorization Roles vs Experience Levels Matrix

| Dimension | Managed By | Examples | Purpose |
|---|---|---|---|
| **Identity** | Cryptographic Substrate | Root `ActorID`, Device Sub-Keys | Cryptographic attribution & signature verification |
| **Authorization Role** | Capability Tokens (`CapabilityProof`) | Owner, Administrator, Member, Guest, Child, Device | Enforces what operations (`OP_READ`, `OP_WRITE`, etc.) are permitted |
| **Experience Level** | UI Shell / Client Preferences | Simple, Standard, Advanced, Expert | Controls visual density and progressive disclosure staging |

---

## 6. Settings Architecture: Global Defaults vs Contextual Controls

Settings in NEX are structured into an **Information Architecture of Consequence**, operating at two distinct tiers:

```text
                              SETTINGS ARCHITECTURE
                                        │
           ┌────────────────────────────┴────────────────────────────┐
           ▼                                                         ▼
    GLOBAL SETTINGS                                         CONTEXTUAL SETTINGS
(Establishes Baseline Defaults)                             (Immediate Subject Controls)
 • You (Profile, Privacy, Security)                          • Photo Settings (Sharing, Backup)
 • Your Nex (Spaces, Devices, Storage, Sync)                 • Person Settings (Trust, Caps, Chat)
 • Applications (Drive, Photos, Chat, Vault)                 • Device Settings (Sync, Cert, CRL)
 • System (Notifications, Appearance, Data)                  • Space Settings (Members, Policies)
 • Advanced (SMT, WAL, Transports, Logs)
```

---

## 7. Universal Concepts Grammar

Every surface in NEX (Drive, Photos, Chat, Vault, Backup, Maps, etc.) MUST recognize and honor the exact same 12 concepts:

1. **Person:** A human sovereign actor identified by a verified or introduced identity.
2. **Identity:** Cryptographic root authority (`ActorID`) and associated device sub-keys.
3. **Device:** A physical hardware host holding a delegated `DeviceCertificate`.
4. **Space:** A human context partition (*Personal, Family, Work, Community, Project*).
5. **Object:** A universal content-addressed or mutable item in the sovereign state DAG.
6. **Permission:** An attenuated capability grant authorizing specific operations.
7. **Trust:** The degree of verification established with a remote peer (Verified, Introduced, Unknown).
8. **Connection:** The active transport channel linking devices (Local WiFi, P2P, Relay).
9. **Storage:** The physical byte allocations and CAS chunks across the local and mesh nodes.
10. **Synchronization:** The anti-entropy reconciliation of causal state DAGs.
11. **Sharing:** The issuance of capability proofs allowing other actors to access objects.
12. **Activity:** The chronological, causal event stream of verified state mutations.
