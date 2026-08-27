# NEX-UX-SOVEREIGNTY-PATTERNS: Visualizing Ownership, Replicas, Trust & Capabilities Without Cryptographic Jargon

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Research Document  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. The Core Sovereignty UX Dilemma

Decentralized and cryptographic software has historically failed at human UX by forcing users to become distributed systems administrators. Users are presented with hexadecimal public keys, Merkle branch depths, erasure coding polynomials, and Byzantine fault descriptions.

**NEX UX Doctrine:**
> *Sovereignty is not an intellectual burden to impose upon the user; it is an emotional feeling of calm, permanence, and unambiguous control.*

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   CRYPTOGRAPHIC TRUTH vs. HUMAN UX GRAMMAR                       │
├────────────────────────────────┬─────────────────────────────────────────────────┤
│ Cryptographic / Substrate Term │ Human-Facing NEX Visual Expression              │
├────────────────────────────────┼─────────────────────────────────────────────────┤
│ Root `ActorID` (Ed25519)       │ "Your Sovereign Identity" / Verified Avatar     │
│ `DeviceCertificate`            │ "Paired Hardware" (e.g. "📱 Chris's Phone")     │
│ `CapabilityProof` Token        │ "Access Pass" (Owner, Member, Guest)            │
│ Attenuated Ops (`OP_READ`)     │ Plain-English permissions ("Can View & Save")   │
│ Sparse Merkle Tree (SMT) Root  │ "Verified Integrity" (🟢 Tamper-proof)          │
│ Content-Addressed CAS Chunks   │ "Encrypted Vault Storage"                       │
│ P2P Anti-Entropy Sync          │ "Physical Device Replicas" (e.g. "Safe on 3")   │
│ CRL / Revocation Vector        │ "Revoke Access Immediately"                     │
└────────────────────────────────┴─────────────────────────────────────────────────┘
```

---

## 2. Communicating Data Residency: The "Safe On N" Pattern

Non-technical users do not care about DHT lookups or socket protocols; they need to know:
1. *Does this file actually exist on my physical hardware?*
2. *If my house burns down or I lose my phone, is my file safe?*

### The Replica Status Hierarchy `[NEX-specific]`

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ 🟢 SAFE ON 3 DEVICES                                                    │
│ ├─ 📱 This Phone (Pixel 9 Pro) — 100% Stored Locally                    │
│ ├─ 💻 Studio Desktop — Synced 2m ago (Verified via Home WiFi)           │
│ └─ 🏡 Living Room Backup Node — Synced 10m ago (Verified via Home WiFi) │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│ 🟡 LOCAL ONLY (1 REPLICA)                                               │
│ ├─ 📱 This Phone (Pixel 9 Pro) — 100% Stored Locally                    │
│ └─ ⚠️ Not yet backed up to another device. Connect to WiFi or Desktop.   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Visual Guidelines for Replicas:
- `[Observed]`: Avoid vague cloud icons (☁️) which imply corporate server custody.
- `[NEX-specific]`: Use explicit device icons (📱 Phone, 💻 Laptop, 🖥️ Desktop, 🏡 Home Node).
- `[Inferred]`: Always display the verified replica count. If a file is only on the current phone, use a gentle amber notice: *"Only on this device. Replicate to your Home Node or Laptop for safety."*

---

## 3. Capability UX: Explicit, Attenuated, Plain-Language Sharing

NEX strictly forbids ambient authority. Every action requires a `CapabilityProof`. However, the user must never be asked to write or parse a token.

### The Human Capability Modal

When a user shares an object (e.g., a photo, a document folder, or a calendar), NEX presents an intuitive capability creator:

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ 🛡️ SHARE "Lake Tahoe Vacation 2026"                                     │
├─────────────────────────────────────────────────────────────────────────┤
│ Who do you want to share with?                                          │
│ [ 👥 Select Person: Amy (Verified Family 🌟)                ▼ ]         │
├─────────────────────────────────────────────────────────────────────────┤
│ What can Amy do?                                                        │
│ ( ) 👁️ CAN VIEW ONLY                                                    │
│     Can view and download local copies. Cannot edit or re-share.        │
│ (•) ✏️ CAN COLLABORATE (Recommended)                                    │
│     Can add photos, edit captions, and share with other family members. │
│ ( ) 👑 FULL CO-OWNERSHIP                                                │
│     Equal sovereign authority, including deletion and permission grants.│
├─────────────────────────────────────────────────────────────────────────┤
│ Duration & Boundaries:                                                  │
│ ⏱️ Access Duration: [ No Expiration                          ▼ ]         │
│ 📡 Allowed Channels: [ Any Connection (WiFi Direct / LAN / Relay) ▼ ]   │
├─────────────────────────────────────────────────────────────────────────┤
│ [ Cancel ]                                         [ ✨ Grant Access ]  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Instant Revocation Feedback `[NEX-specific]`
When the user clicks "Revoke Access", the UI immediately reflects the revocation in the local state, issues a cryptographic CRL entry, and marks the capability token invalid. The UI communicates: *"Revoked. Amy's devices will no longer receive updates or verify access."*

---

## 4. Trust Verification UX: The Social Authentication Flow

Establishing trust between two humans must be tactile and verifiable without requiring trust in a third-party certificate authority (CA).

### The Proximity SAS (Short Authentication String) Flow

```text
  [ Person A (Phone) ]                      [ Person B (Phone) ]
          │                                         │
          ▼                                         ▼
 ┌──────────────────────┐                  ┌──────────────────────┐
 │ Scan Amy's QR Code   │ ──(Camera Scan)─▶│ Amy's Trust Screen   │
 │                      │                  │                      │
 │   [ 📷 Camera ]      │                  │   ┌──────────────┐   │
 │                      │                  │   │  ████  ████  │   │
 │                      │                  │   │  ████  ████  │   │
 │                      │                  │   └──────────────┘   │
 └──────────────────────┘                  └──────────────────────┘
          │                                         │
          ▼                                         ▼
 ┌────────────────────────────────────────────────────────────────┐
 │ 🛡️ VERIFY SAFETY WORDS                                         │
 │ Compare these 4 words with Amy in person:                      │
 │                                                                │
 │     🌲 RIVER    ⛰️ SUMMIT    🦊 FALCON    🌊 HARBOR            │
 │                                                                │
 │ [ Words Do Not Match ]                   [ ✅ Confirm & Trust ] │
 └────────────────────────────────────────────────────────────────┘
```

### Trust Badges Across the Ecosystem:
- 🟢 **Verified Human (Shield with Check):** Proximity SAS or QR code verified in person. Full capability delegation enabled.
- 🔵 **Introduced via Family (Linked Shield):** Introduced by a trusted family member. Read capabilities enabled.
- ⚪ **Local Peer (Plain Outline):** Discovered on the local Wi-Fi network; unknown identity. No automatic access granted.

---

## 5. Summary of Sovereignty UX Invariants

1. **No Ambient Access `[NEX-specific]`:** No user or app has access to any object without a visible, delegatable, and revocable capability badge.
2. **Physical Over Abstract `[Inferred]`:** Data residency is always anchored to physical hardware devices, not abstract cloud servers.
3. **Calm Transparency `[Observed]`:** Security is conveyed through clear status badges and plain language, never alarming technical jargon.
