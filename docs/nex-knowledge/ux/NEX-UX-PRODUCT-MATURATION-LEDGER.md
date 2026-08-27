# NEX-UX-PRODUCT-MATURATION-LEDGER: 15 Human Journeys, Adversarial Truthfulness Audit & Trial Instrumentation

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Product Maturation Specification (Baseline Frozen at Human Product Baseline v1)  
**Governance Hierarchy:** Level 1–2 Constitution & Frozen Wire/WAL → Level 3–4 ADRs & Sealed Gates → Level 4 NEX-UX-01 → UX Research Baseline → Human Product Baseline v1 → Maturation & Trial Evidence  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. Freezing Human Product Baseline v1

In accordance with the Post-L4/L5 Human Trial Directive, the current native UI implementation is officially frozen as **Human Product Baseline v1**:

```text
┌─────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                     HUMAN PRODUCT BASELINE v1 (FROZEN)                                          │
├───────────────────────────┬─────────────────────────────────────────────────────────────────────────────────────┤
│ Visual Language           │ Calm Sovereignty / Native Precision Hybrid (Obsidian `#121216`, Slate `#22222B`)     │
│ Information Architecture  │ Dual-Axis Spaces (Context) × Lenses (Perspective)                                   │
│ Universal Grammar         │ 1 Logical Object in DAG ──▶ Multi-Lens Projections (Photos, Drive, Media, Maps, etc)│
│ Universal Inspector       │ 4 Sections: Identity/Provenance, Capabilities/Access, Physical Residency, Diagnostics│
│ Physical Residency        │ "Safe on N Devices" explicitly paired with physical byte breakdown per host         │
│ Truthful Status Engine    │ 4-State Matrix: Local Only (🟡), Replicating (🔵), Verified (🟢), Offline Queued (⚪)│
│ Experience Slider         │ 4 Staged Tiers: Simple (🟢), Standard (🔵), Advanced (🟡), Operator (🟣)             │
│ Proximity Trust           │ 4-Word Safety String SAS Ceremony ("River • Summit • Falcon • Harbor")              │
│ Brand Integration         │ Interlocking N/X symbol restricted to App Identity & Topology Hubs (No icon spoofing)│
└───────────────────────────┴─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. The 15 Real Human Journeys Audit

To move from a "validated vertical slice" toward a "usable product," the system is tested across 15 real personal computing workflows:

```text
┌────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                       15 REAL HUMAN JOURNEYS AUDIT MATRIX                                              │
├──────┬──────────────────────────────┬──────────────────────────────┬─────────────────────────────┬─────────────────────┤
│ #    │ Journey Description          │ Human User Task & Action     │ Required UI Expression      │ Substrate Grounding │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 01   │ **Photo $\to$ Share w/ Amy** │ Drop JPEG $\to$ click Share  │ Capability sheet: "Can View"│ Attenuated CapProof │
│      │                              │ $\to$ select Amy from family │ $\to$ Amy added to access   │ signed by ActorID   │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 02   │ **Amy Accesses Photo**       │ Amy opens Family Space on    │ Appears in Amy's Photos     │ SMT anti-entropy    │
│      │                              │ her paired phone             │ feed with 🟢 Verified badge │ DAG replication     │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 03   │ **Internet Outage**          │ Router unplugged; user       │ "Operating Locally (No Net)"│ Local WAL write     │
│      │                              │ edits photo caption & tags   │ $\to$ saves with zero delay │ + outbox queuing    │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 04   │ **Device Disconnection**     │ Laptop battery dies during   │ TopBar shows "Degraded:     │ TAL socket timeout  │
│      │                              │ background replication       │ 1 Peer Offline" (calmly)    │ event handled       │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 05   │ **Truthful Recovery**        │ Laptop boots $\to$ auto P2P  │ TopBar pulses blue $\to$    │ Anti-entropy batch  │
│      │                              │ reconnects on local Wi-Fi    │ turns solid green (Verified)│ Merkle root match   │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 06   │ **Create Folder / Album**    │ Group photos into "Tahoe"    │ Album created in Photos;    │ Logical collection  │
│      │                              │ collection                   │ visible as folder in Drive  │ tag in object meta  │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 07   │ **Move Between Spaces**      │ Move document from Personal  │ Space badge changes $\to$   │ `NamespaceID`       │
│      │                              │ Space to Family Space        │ access policy updates       │ re-keyed in DAG     │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 08   │ **Revoke Peer Access**       │ Revoke Guest access on shared│ Immediate strike-through    │ SMT CRL entry       │
│      │                              │ project folder               │ $\to$ "Revoked locally"     │ committed to state  │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 09   │ **Add New Trusted Device**   │ Pair new tablet via SAS      │ Scan QR $\to$ verify 4 words│ Master Key delegates│
│      │                              │ camera scan                  │ $\to$ tablet added to mesh  │ DeviceCertificate   │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 10   │ **Device Loss Recovery**     │ Phone dropped in lake;       │ Pair replacement phone $\to$│ Social Shamir share │
│      │                              │ restore from Home Node       │ all DAG data syncs over LAN │ + Timelock recovery │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 11   │ **Universal Omnisearch**     │ Search "Tahoe" from TopBar   │ Returns photo tile, drive   │ Global multi-lens   │
│      │                              │ without choosing a Lens      │ file & map marker in 1 list │ DAG index query     │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 12   │ **Export Personal Data**     │ Export photo as local JPEG   │ Native file dialog $\to$    │ CAS payload stream  │
│      │                              │ to USB drive                 │ writes exact original bytes │ to OS filesystem    │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 13   │ **Backup Snapshot**          │ Trigger snapshot to external │ "Backup Created (Snapshot   │ Encrypted CAS chunk │
│      │                              │ offline hard drive           │ #14) — 100% Encrypted"      │ manifest export     │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 14   │ **Restore from Snapshot**    │ Rebuild state from cold-     │ Progress bar $\to$ restores │ CAS chunk unpack    │
│      │                              │ storage USB backup           │ all objects with zero loss  │ + SMT verification  │
├──────┼──────────────────────────────┼──────────────────────────────┼─────────────────────────────┼─────────────────────┤
│ 15   │ **Inspect Physical Storage** │ Check where 4.2 MB photo     │ Universal Inspector details │ Real CAS byte audit │
│      │                              │ exists physically            │ Phone, PC & Home Node disk  │ across active nodes │
└──────┴──────────────────────────────┴──────────────────────────────┴─────────────────────────────┴─────────────────────┘
```

---

## 3. Adversarial Truthfulness Audit

We specifically attack every user-facing claim to guarantee that the UI never implies something stronger than the substrate mathematically guarantees:

```text
┌─────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                    ADVERSARIAL TRUTHFULNESS LEDGER                                              │
├───────────────────────────┬──────────────────────────────────┬──────────────────────────────────────────────────┤
│ Claimed Subsystem         │ Potential False Assumption Attack│ Enforcement & Protection Rule                    │
├───────────────────────────┼──────────────────────────────────┼──────────────────────────────────────────────────┤
│ **1. Synchronization**    │ UI reports "Synced" when chunks  │ FORBIDDEN. Green badge requires acknowledged SMT │
│                           │ are only buffered in local RAM.  │ Merkle root match from remote peer.              │
├───────────────────────────┼──────────────────────────────────┼──────────────────────────────────────────────────┤
│ **2. Physical Storage**   │ User assumes "1 Object in DAG"   │ FORBIDDEN. UI must show logical size (4.2 MB) vs │
│                           │ means replicas use zero disk.    │ physical disk consumption on each device.        │
├───────────────────────────┼──────────────────────────────────┼──────────────────────────────────────────────────┤
│ **3. Revocation**         │ User assumes clicking Revoke     │ FORBIDDEN. UI must state: "Revocation active     │
│                           │ deletes data from airgapped peer.│ locally; applies upon next mesh contact."        │
├───────────────────────────┼──────────────────────────────────┼──────────────────────────────────────────────────┤
│ **4. Encryption at Rest** │ User assumes local cleartext     │ FORBIDDEN. UI must truthfully state if volume is │
│                           │ is hardware-TEE encrypted.       │ cleartext, OS Keystore, or Zero-Knowledge Vault. │
├───────────────────────────┼──────────────────────────────────┼──────────────────────────────────────────────────┤
│ **5. Peer Online State**  │ UI assumes peer is online because│ FORBIDDEN. Status reverts to "Degraded / Offline"│
│                           │ mDNS announced 10 minutes ago.   │ if active heartbeat drops past 15 seconds.       │
└───────────────────────────┴──────────────────────────────────┴──────────────────────────────────────────────────┘
```

---

## 4. Human Trial Instrumentation Framework

We decouple engineering correctness from human comprehension and emotional confidence:

```text
                                 THE THREE-TIER TRIAL METRIC
                                              │
         ┌────────────────────────────────────┼────────────────────────────────────┐
         ▼                                    ▼                                    ▼
[ ENGINEERING TRUTH ]               [ HUMAN COMPREHENSION ]               [ HUMAN DELIGHT ]
 • Rust tests 82/82 pass             • Task completed unaided              • Zero anxiety
 • CRC32 / WAL frames valid          • Zero wrong navigation clicks        • Feels safe & durable
 • SMT roots mathematically match    • Explanations required = 0           • "That's my stuff"
```

### Tracked Human Trial Metrics:
1. **Unassisted Task Completion Rate (Goal: >95%):** Non-technical users completing journeys 01–15 without intervention.
2. **Zero-Jargon Comprehension (Goal: 100%):** Zero questions asked about SMT, CAS, CRDT, or Lamport clocks.
3. **Mistaken Dependency Rate (Goal: 0%):** Zero users believing they need an internet connection to open their files or pair local devices.
