# NEX-UX-LABORATORY-PHASE-1: Visual Laboratory Experiments, 4-Direction Stress Testing & Human Journey Evaluation

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Experimental Evaluation Report (Phase 1 Laboratory Deliverable)  
**Governance Precedence:** Level 1–2 Constitution & Frozen Wire/WAL → Level 3–4 ADRs & Sealed Gates → Level 4 NEX-UX-01 → UX Research Baseline (Subordinate) → Figma Laboratory Experiments  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. Executive Summary & Laboratory Scope

The Phase 1 Visual Laboratory was built to transition NEX from theoretical UI/UX research to concrete, visual and interactive experimentation. In accordance with constitutional discipline, no native desktop implementation was mutated. Instead, the visual laboratory established a reusable, token-backed prototyping system across the five mandatory functional areas:

1. **Tokens:** Semantic variable collections (Color, Typography, 4px Spacing Grid, Corner Radii, Elevation).
2. **Components:** Reusable atomic primitives (Status Pills, Trust Shields, Object Cards, Contact Cards, Device Tiles, Universal Object Inspector Drawer).
3. **Four Visual Directions:** Full visual language models for Directions A, B, C, and D.
4. **Canonical Human Journey:** End-to-end simulation of the 8-step flow (*Home → Family → Photos → Photo → Inspector → Person → Device → Experience Slider*).
5. **Experience Slider Complexity Stress Test:** All 4 presentation tiers (*Simple 🟢 → Standard 🔵 → Advanced 🟡 → Operator 🟣*) rendered under real degraded and offline states.

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                         PHASE 1 VISUAL LABORATORY ARCHITECTURE                         │
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                 NEX DESIGN SYSTEM TOKENS                               │
│            [ 4px Grid ]   [ Semantic Colors ]   [ 8-16px Radii ]   [ Typography ]      │
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                REUSABLE ATOMIC COMPONENTS                              │
│       [ Status Pills ]   [ Trust Shields ]   [ Cards ]   [ Universal Inspector ]       │
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                4 COMPETING DESIGN DIRECTIONS                           │
│  Direction A (Calm) │ Direction B (Native) │ Direction C (Spatial) │ Direction D (Util)│
├────────────────────────────────────────────────────────────────────────────────────────┤
│                           THE CANONICAL HUMAN JOURNEY PROTOTYPE                        │
│   Home ──▶ Family Space ──▶ Photos ──▶ Photo ──▶ Inspector ──▶ Amy ──▶ Device ──▶ Slider│
├────────────────────────────────────────────────────────────────────────────────────────┤
│                         THE 4-TIER COMPLEXITY MATRIX STRESS TEST                       │
│    Simple (🟢)    │    Standard (🔵)    │    Advanced (🟡)    │    Operator (🟣)       │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Laboratory Setup & Variable Token Hierarchy

The laboratory relies on a decoupled 3-tier variable architecture, ensuring that changing a radius, spacing step, or surface tint propagates instantly without requiring manual canvas rebuilding.

```text
  ┌───────────────────────────────────────────────────────────────────┐
  │ GLOBAL PRIMITIVES                                                 │
  │ • color.slate.950 = #121216     • radius.md = 8px                 │
  │ • color.slate.900 = #18181E     • radius.lg = 12px                │
  │ • color.slate.800 = #22222B     • space.base = 4px                │
  └─────────────────────────────────┬─────────────────────────────────┘
                                    │ Aliased to Semantic Meaning
                                    ▼
  ┌───────────────────────────────────────────────────────────────────┐
  │ SEMANTIC SYSTEM TOKENS (Shared across all 4 Directions)           │
  │ • surface.canvas                • status.sync.local (🟡 #FBBF24)  │
  │ • surface.panel                 • status.sync.replicating (🔵)    │
  │ • surface.sidebar               • status.sync.verified (🟢 #34D399│
  │ • trust.verified (🟢 Shield)     • status.sync.offline (⚪ #9CA3AF) │
  └─────────────────────────────────┬─────────────────────────────────┘
                                    │ Consumed by Components
                                    ▼
  ┌───────────────────────────────────────────────────────────────────┐
  │ COMPONENT INSTANCES                                               │
  │ • UniversalObjectInspector      • DeviceContextTile               │
  │ • PersonContactCard             • TruthfulSyncBadge               │
  └───────────────────────────────────────────────────────────────────┘
```

---

## 3. Four-Direction Comprehensive Evaluation Matrix

Each of the four directions was subjected to the identical canonical journey and evaluated along the five constitutional UX criteria: **Comprehension, Confidence, Sovereignty, Continuity, and Complexity Control**.

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                             FOUR-DIRECTION COMPARATIVE LEDGER                                                    │
├───────────────────┬────────────────────────────────┬────────────────────────────────┬────────────────────────────────┬───────────┤
│ Dimension         │ Direction A: Calm Sovereignty  │ Direction B: Modern Native     │ Direction C: Spatial Object    │ Dir D:    │
│                   │                                │                                │                                │ Minimal   │
├───────────────────┼────────────────────────────────┼────────────────────────────────┼────────────────────────────────┼───────────┤
│ Base Materiality  │ Warm Obsidian (`#121216`),     │ Cool Slate (`#131418`),        │ Coordinate Dot-Grid Plane,     │ Pure Dark │
│                   │ Slate panels (`#22222B`),      │ Translucent Acrylic Glass,     │ Floating Canvas Cards,         │ (`#0A0A0C`│
│                   │ 12px pill radii, warm glow     │ 1px Hairline White Borders     │ Dynamic SVG Connection Strands │ 1px border│
├───────────────────┼────────────────────────────────┼────────────────────────────────┼────────────────────────────────┼───────────┤
│ Typography        │ Humanist Sans (`Inter`) +      │ Platform Sans (`SF Pro` /      │ Modern Sans (`Geist` /         │ Monospace │
│                   │ Editorial Serif (`Newsreader`) │ `Segoe UI Variable`)           │ `Inter`) + Vector Micro-Tags   │ (`SF Mono`│
├───────────────────┼────────────────────────────────┼────────────────────────────────┼────────────────────────────────┼───────────┤
│ Key Strength      │ Highest emotional reassurance; │ Instant familiarity for macOS/ │ Makes multi-lens data sharing  │ Highest   │
│                   │ feels permanent & tactile      │ Windows desktop power users    │ visually obvious & physical    │ density   │
├───────────────────┼────────────────────────────────┼────────────────────────────────┼────────────────────────────────┼───────────┤
│ Primary Weakness  │ Serif headers feel too formal  │ Glassmorphism can dilute high- │ Canvas navigation causes visual│ Intimidati│
│                   │ in dense diagnostic views      │ density diagnostic legibility  │ noise & spatial disorientation │ to non-tec│
├───────────────────┼────────────────────────────────┼────────────────────────────────┼────────────────────────────────┼───────────┤
│ Best Persona      │ Families, writers, creators,   │ General desktop users,         │ Researchers, systems thinkers, │ Terminal  │
│                   │ privacy-conscious individuals  │ multi-window multitaskers      │ visual node explorers          │ operators │
├───────────────────┼────────────────────────────────┼────────────────────────────────┼────────────────────────────────┼───────────┤
│ Biggest Risk      │ Might look too "lifestyle-app" │ Might be mistaken for a        │ High GPU rendering cost; hard  │ Too stark │
│                   │ if expert depth is softened    │ standard cloud SaaS wrapper    │ for linear everyday workflows  │ for photos│
├───────────────────┼────────────────────────────────┼────────────────────────────────┼────────────────────────────────┼───────────┤
│ Strongest NEX     │ "Safe on 3 Replicas" tactile   │ Fluid OS-native window & tray  │ Visual DAG linking Amy + Phone │ Absolute  │
│ Trait Expressed   │ badge feels like a home safe   │ integration with file picker   │ + Photo in one visible canvas  │ data speed│
└───────────────────┴────────────────────────────────┴─────────────────────────────┴────────────────────────────────┴───────────┘
```

---

## 4. The Canonical Journey Experiment: Step-by-Step Findings

We walked the canonical journey through all four design directions:

```text
[01. Home Arrival] ──▶ [02. Family Space] ──▶ [03. Photos Lens] ──▶ [04. Open Photo]
        │
        ▼
[05. Slide Open Universal Inspector] ──▶ [06. Inspect Amy's Card] ──▶ [07. Inspect Device Tile]
        │
        ▼
[08. Toggle Experience Slider: Simple 🟢 ──▶ Standard 🔵 ──▶ Advanced 🟡 ──▶ Operator 🟣]
```

### Screen 1: Home Arrival & Space Selection
- **What the user needs to know:** *"I am in my private space. My system is healthy. My family space is 1 click away."*
- **What the user has the ability to know:** Active node ActorID, local port bindings, active peer count.
- **Finding `[NEX-specific]`:** Direction A and Direction B both succeeded in placing the Space switcher (*Personal, Family, Work, Community*) in the top primary frame without cluttering the viewport. Direction C felt overly abstract on initial arrival.

### Screen 2: Photos Lens & Photo Ingestion
- **What the user needs to know:** The photo is saved immediately. It is safe.
- **What the user has the ability to know:** FastCDC chunk hashes, CAS byte offset, Lamport timestamp.
- **Finding `[Observed]`:** When an image is dropped into NEX, users immediately look for a status indicator on the thumbnail. A gentle bottom-right badge (🟡 *Local Only* or 🟢 *Safe on 3*) gives immediate emotional closure.

### Screen 3: The Universal Object Inspector (The Centerpiece of Sovereignty)
- **What the user needs to know:**
  1. *Where does this file live physically?* (`🟢 Safe on 3 Devices: Phone, PC, Home Node`).
  2. *Who can see it?* (`Amy: Member — Can View & Add Photos`).
- **What the user has the ability to know:**
  - SMT Leaf Hash, CAS Hash, Capability Token Signature, WAL Frame Sequence Number.
- **Finding `[NEX-specific]`:** The Universal Inspector is the single most powerful architectural surface in NEX. When users see the *Physical Residency* section explicitly listing their phone and desktop, the abstract concept of "local-first" becomes instantly concrete.

### Screen 4: Person Surface (Amy)
- **What the user needs to know:** Amy is a verified contact. We have 42 shared family photos.
- **What the user has the ability to know:** Amy's Ed25519 Public Key, Delegated Sub-Keys, SAS QR safety words.
- **Finding `[Observed]`:** Displaying a 🟢 *Verified Family* shield with Amy's avatar communicates complete security without showing 64 hexadecimal characters.

### Screen 5: Device Surface (Studio Desktop & Living Room Node)
- **What the user needs to know:** Desktop is connected via High-Speed Home Wi-Fi (120 MB/s). It has 100% of my family photos.
- **What the user has the ability to know:** Direct socket IP, TAL packet drop rate, Storage Quota CAS allocations.
- **Finding `[Inferred]`:** Showing direct LAN transfer speed (*"Direct WiFi Direct Mesh — 120 MB/s"*) reinforces that data is flowing inside the house, not through a third-party cloud.

---

## 5. Prototype Real-State Evaluations (Degraded & Offline Conditions)

The laboratory explicitly stress-tested the edge states that break traditional cloud apps:

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                            PROTOTYPED REAL-STATE MATRIX                                │
├───────────────────────────┬────────────────────────────────┬───────────────────────────┤
│ Scenario                  │ Primary UI Visual Expression   │ Progressive Disclosure    │
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Photo: Local Only         │ 🟡 Amber dot on thumbnail      │ Inspector: "Stored on this│
│                           │ "Saved on this phone only"     │ phone. Connect to WiFi."  │
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Photo: Replicating (1 of 3│ 🔵 Pulsing blue sync pill      │ Inspector: "Transferring  │
│                           │ "Syncing with Desktop (45%)"   │ chunk 3/8 over Home WiFi" │
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Photo: Synchronized       │ 🟢 Solid emerald dot           │ Inspector: "Verified SMT  │
│                           │ "Safe on 3 Replicas"           │ Root on Phone, PC & Node" │
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Photo: Offline Queued     │ ⚪ Calm white outline dot      │ Inspector: "Queued in     │
│                           │ "Offline. Will sync on connect"│ Outbox (Retry backoff 5s)"│
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Device: Offline           │ ⚪ Muted gray outline tile     │ Device Panel: "Last seen  │
│                           │ "Studio Desktop (Offline 2h)"  │ 2 hours ago via Home LAN" │
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Network: LAN Direct       │ 🟢 "Local Home Network Direct" │ Network Panel: "Zero-hop  │
│                           │ "No Internet required"         │ TCP direct socket"        │
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Network: Mesh Ad-Hoc      │ 🔵 "Direct Phone-to-Laptop"    │ Network Panel: "WiFi-Dir  │
│                           │ "Syncing in field (No Router)" │ P2P channel active"       │
├───────────────────────────┼────────────────────────────────┼───────────────────────────┤
│ Network: Airgapped / None │ ⚪ "Operating Offline"         │ Outbox Panel: "14 changes │
│                           │ "All features fully functional"│ journaled locally in WAL" │
└───────────────────────────┴────────────────────────────────┴───────────────────────────┘
```

---

## 6. The "Magic Moment" Discovery: Multi-Lens Object Continuity

The visual laboratory deliberately engineered and validated the **NEX Magic Moment**:

```text
                                [ OBJECT INGESTION ]
                            User imports "Sunset.jpg"
                                       │
            ┌──────────────────────────┼──────────────────────────┐
            ▼                          ▼                          ▼
    [ FAMILY SPACE ]            [ PHOTOS LENS ]            [ PERSON: AMY ]
   Appears in Family          Appears in Timeline        Appears under Amy's
    Feed immediately           as PhotoMedia tile         Shared Asset Feed
            │                          │                          │
            └──────────────────────────┼──────────────────────────┘
                                       │
                                       ▼
                       [ UNIVERSAL OBJECT INSPECTOR ]
                 "One single object in your sovereign DAG.
                  Zero duplicated bytes. 3 Physical Replicas."
```

### The Human Realization `[NEX-specific]`
When a user sees the same photo in **Family**, then clicks **Amy** and sees it in her shared stream, then opens **Drive** and sees it in `Family/Vacations/`, and then opens the **Inspector** to confirm that it is **one single entity stored once on their local disk**, they have the breakthrough realization:

> **“Oh. NEX isn't five different cloud apps syncing in the background. It's my single private world, viewed through different magnifying glasses.”**

This is the exact emotional and cognitive turning point of the NEX UX.

---

## 7. Experience Slider Evaluation: What Survives Across All 4 Levels

We evaluated what remains visible versus what is staged behind progressive disclosure:

```text
┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                 EXPERIENCE SLIDER DISCLOSURE MATRIX                               │
├──────────────────────────────┬───────────────┬─────────────────┬────────────────┬─────────────────┤
│ Information Item             │ 🟢 Simple     │ 🔵 Standard     │ 🟡 Advanced    │ 🟣 Operator     │
├──────────────────────────────┼───────────────┼─────────────────┼────────────────┼─────────────────┤
│ Object Thumbnail & Title     │ Visible       │ Visible         │ Visible        │ Visible         │
│ Space Context (Family)       │ Badge         │ Dropdown Bar    │ Dropdown Bar   │ Filter Query    │
│ "Safe on N" Replica Status   │ "Safe on 2"   │ "Safe on 3"     │ Detailed Hosts │ Merkle Node DAG │
│ Plain Capability Access      │ "Amy can view"│ "Amy (Member)"  │ Explicit Tokens│ Ed25519 CapProof│
│ Network Connection State     │ "Online"      │ "Home Wi-Fi"    │ "LAN 120 MB/s" │ Socket IP & RTT │
│ Outbox Synchronization Queue │ Hidden        │ Badge Count (2) │ Full Queue List│ WAL Transaction │
│ SMT Root Hash & Leaf Path    │ Hidden        │ Hidden          │ Short Hash     │ 256-bit Hex Root│
│ FastCDC Chunk Boundaries     │ Hidden        │ Hidden          │ Hidden         │ 4 Chunks / CAS  │
│ Erasure Coding Matrix (K/M)  │ Hidden        │ Hidden          │ Hidden         │ 4+2 Cauchy RS   │
└──────────────────────────────┴───────────────┴─────────────────┴────────────────┴─────────────────┘
```

### Key Experience Slider Discovery `[NEX-specific]`
The product remains **100% recognizably NEX** at all four levels. The layout geometry, navigation bar, and core object cards never shift or disappear. Only the **density of explanatory text and diagnostic depth inside containers** expands.

---

## 8. Architectural Tensions & Findings

During the laboratory experiments, we identified three genuine tensions where human UX desires interact with substrate boundaries:

### Tension 1: Instant Local Creation vs. Remote Peer Replica Lag `[Inferred]`
- **Human Expectation:** User imports a 50MB video; they want to see "Synced" immediately because their laptop saved it instantly.
- **Architectural Reality:** Physical Wi-Fi replication takes 400ms. If the UI says "Synced" immediately, it violates the **Truthful State Invariant**.
- **UX Resolution:** Introduce the gentle transient state: *"Saved locally. Syncing with Living Room Node..."* with a subtle blue progress arc.

### Tension 2: Capability Revocation Propagation in Offline Mesh `[NEX-specific]`
- **Human Expectation:** User clicks "Revoke Amy's Access"; they expect Amy's phone to lose access instantly, even if Amy is camping in an airgapped forest.
- **Architectural Reality:** Revocation propagates via the SMT CRL when Amy's device reconnects to any mesh peer.
- **UX Resolution:** The UI truthfully states: *"Revocation active locally. Access will be blocked on Amy's devices upon next mesh contact."*

### Tension 3: Experience Slider Complexity vs. Accidental Mode Trapping `[Experimental]`
- **Human Expectation:** A user toggles to "Operator" to inspect a Merkle hash, then forgets how to return to "Standard".
- **UX Resolution:** Place the Experience Slider persistently in the global top-right header with an explicit, human-readable indicator (*"Experience: Standard ▾"*).

---

## 9. Final Synthesis & Recommended Direction

### The Decision: A Deliberate Hybrid of Direction A and Direction B

The visual laboratory demonstrated that neither pure Direction A nor pure Direction B is 100% optimal in isolation:
- Pure **Direction A (Calm Sovereignty)** delivers exceptional human warmth and tactile reassurance, but its editorial serif typography becomes too heavy in dense diagnostic tables.
- Pure **Direction B (Modern Native)** delivers razor-sharp desktop ergonomics, but its cool acrylic glassmorphism lacks the warm, heirloom permanence of a sovereign personal vault.

### The Recommended Master Language: **"Calm Sovereignty (Native Precision Hybrid)"**
1. **Surfaces & Atmosphere:** Warm Obsidian canvas (`#121216`) paired with soft slate panels (`#22222B`) and 12px pill radii (from Direction A).
2. **Typography:** Crisp Humanist Sans (`Inter` / `Source Sans 3`) across all interface text and data tables, reserving warm editorial serif accents exclusively for primary Space welcoming headers (e.g. *"Welcome to Family Space"*).
3. **Sovereignty Badging:** Tactile physical replica pills (*"🟢 Safe on 3 Devices"*) and emerald SAS trust shields (*"🟢 Verified Human"*).
4. **Window Ergonomics:** Clean native desktop titlebar and persistent sidebar layout (from Direction B).

---

## 10. Implementation Phase Roadmap (What Moves to Code)

With Phase 1 laboratory validation complete, the following changes are prepared for the native desktop implementation (`nex-desktop`):

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                            DESKTOP REALIZATION ROADMAP                                 │
├───────────────────────┬────────────────────────────────────────────────────────────────┤
│ Target Subsystem      │ Planned Native Realization                                     │
├───────────────────────┼────────────────────────────────────────────────────────────────┤
│ 1. Palette & Styles   │ Update `nex-desktop/src/ui/palette.rs` to Warm Obsidian tokens.│
│ 2. Universal Inspector│ Implement the multi-section sliding drawer (Residency, Caps).  │
│ 3. Truthful Sync Pill │ Bind TopBar sync pill to SMT anti-entropy state machine.       │
│ 4. Proximity SAS Modal│ Add 4-word safety string confirmation to `actions.rs`.         │
│ 5. Slider Persistence │ Retain 4-step combobox in TopBar controlling UI density.       │
└───────────────────────┴────────────────────────────────────────────────────────────────┘
```
