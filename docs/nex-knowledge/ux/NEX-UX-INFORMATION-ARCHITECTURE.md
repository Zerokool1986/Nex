# NEX-UX-INFORMATION-ARCHITECTURE: The Dual-Axis "Spaces × Lenses" Model & Universal Object Navigation

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Research Document  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. The Core IA Challenge

Traditional personal computing is split into two broken paradigms:
1. **The Operating System Admin Panel (Filesystem Hierarchy):** Exposes raw disk directories (`C:\Users\Admin\Documents\Photos\2026\IMG_001.jpg`), disk mount points, partition volumes, and process lists. Highly sovereign, but hostile to non-technical users.
2. **The Cloud App Silo:** Segregates data into closed cloud applications (Google Drive vs. Google Photos vs. Slack vs. WhatsApp). User-friendly, but destroys local sovereignty, creates redundant data copies, and forces users to navigate between siloed ecosystems.

NEX solves this via the **Dual-Axis Information Architecture**:
- **Horizontal Context Axis (Spaces):** *Who is this for?* (Personal, Family, Work, Community, Project).
- **Vertical Perspective Axis (Lenses):** *How do I want to see it?* (Home, Photos, Drive, Media, Maps, People, Devices, Network, Settings).

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                     THE DUAL-AXIS INFORMATION ARCHITECTURE (IA)                        │
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                  SPACES (Context Axis)                                 │
│                   [ Personal ]   [ 🌟 Family ]   [ Work ]   [ Community ]               │
│  LENSES         ┌───────────────┬───────────────┬──────────┬─────────────┐             │
│  (Perspective   │               │               │          │             │             │
│   Axis)         │               │               │          │             │             │
│                 ▼               ▼               ▼          ▼             │             │
│  🏠 Home        │ Feed/Recent   │ Family Feed   │ Work Hub │ Announcements│             │
│  📷 Photos      │ Private Snaps │ Family Albums │ Projects │ Shared Drops│             │
│  💾 Drive       │ Personal Docs │ Household/Tax │ Client   │ Public Docs │             │
│  🎬 Media       │ Music/Videos  │ Family Movies │ Training │ Broadcasts  │             │
│  🗺  Maps       │ Saved Places  │ Family Checkin│ Sites    │ Group Pins  │             │
│  👥 People      │ Direct Peers  │ Family Members│ Teammates│ Members     │             │
│  📱 Devices     │ My Phone/PC   │ Living Room TV│ Work Ltp │ Nodes       │             │
│  🌐 Network     │ P2P Topology  │ Home Mesh     │ VPN Wire │ Relay Mesh  │             │
│  ⚙ Settings     │ Privacy       │ Shared Quotas │ Org Cap  │ Space Perms │             │
└─────────────────┴───────────────┴───────────────┴──────────┴─────────────┴─────────────┘
```

---

## 2. The 12 Universal Grammar Concepts in Navigation

Every surface in NEX is constructed from the exact same 12 canonical concepts:

```text
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                                 THE 12 CONCEPTS                                      │
├───────────────────┬──────────────────────────────────┬───────────────────────────────┤
│ Concept           │ Underlying Substrate State       │ Visual UI Representation      │
├───────────────────┼──────────────────────────────────┼───────────────────────────────┤
│ 1. Person         │ Root `ActorID` + Ed25519 Keys    │ Contact Card / Avatar Pill    │
│ 2. Identity       │ Active Device Certificate        │ Identity Badge (ID: 55a8…)    │
│ 3. Device         │ Physical Node ID + TEE Proof     │ Device Tile / Battery / Mesh  │
│ 4. Space          │ `NamespaceID` Partition Filter   │ Top Context Tab Bar           │
│ 5. Object         │ Content-Addressed `NexObject`    │ Unified Card / Gallery Item   │
│ 6. Permission     │ `CapabilityProof` Attenuation    │ Access Badge (Owner/Member)   │
│ 7. Trust          │ SAS QR Verification State        │ Trust Shield (🟢 Verified)    │
│ 8. Connection     │ Active TAL Transport Channel     │ Link Indicator (LAN/BLE/Mesh) │
│ 9. Storage        │ CAS Chunks + Storage Quotas      │ Storage Gauge (Local/Mesh)    │
│ 10. Synchronization│ SMT Root Reconciliation          │ Truthful Sync Pill            │
│ 11. Sharing       │ Capability Delegation Token      │ Recipient Tag Stack           │
│ 12. Activity      │ Verified Causal Event Stream     │ Timeline Feed / Audit Drawer  │
└───────────────────┴──────────────────────────────────┴───────────────────────────────┘
```

---

## 3. The Object-Centric Mental Model

In NEX, an object is **not a file path on a specific hard drive**. An object is an immutable or mutable state node in the user's sovereign DAG.

### The Universal Object Lifecycle
1. **Ingestion `[NEX-specific]`:** A JPEG image is dragged into NEX. FastCDC chunks the bytes into content-addressed CAS blocks. A new `NexObject` of type `PhotoMedia` is minted and signed by the user's active `ActorID`.
2. **Multi-Lens Projection `[Inferred]`:**
   - In **Photos Lens**, it renders as an image tile sorted by timestamp metadata.
   - In **Drive Lens**, it appears as `Lake_Tahoe.jpg` inside the `Family/Vacations/` folder virtual directory.
   - In **Maps Lens**, it appears as a geotagged photo marker at coordinates `(39.0968, -120.0324)`.
   - In **People Lens**, it appears under Amy's shared asset feed.
3. **Zero Data Duplication `[NEX-specific]`:** The storage layer stores the payload once in CAS. The different lenses are simply projection queries over the same Merkle state.

---

## 4. The Universal Object Inspector (Persistent Right Drawer)

Whenever any object, person, or device is selected across any lens, the **Universal Object Inspector** slides into view. It provides full transparency without taking the user out of their workflow.

```text
┌─────────────────────────────────────────────────────────────┐
│ 🔍 UNIVERSAL OBJECT INSPECTOR                               │
├─────────────────────────────────────────────────────────────┤
│ 📷 Integrated Family Photo.jpg                              │
│ Space: Family (🌟) | Type: PhotoMedia | Size: 4.2 MB        │
├─────────────────────────────────────────────────────────────┤
│ 📍 PROVENANCE & IDENTITY                                    │
│ Created: Aug 26, 2026 14:32 by You (📱 Pixel 9 Pro)         │
│ Winning Mutation: 55a8f901… (Lamport: 14)                   │
├─────────────────────────────────────────────────────────────┤
│ 🛡️ CAPABILITIES & ACCESS                                    │
│ • You (Owner) — Full Control                                │
│ • Amy (Member) — Read, Annotate, Re-share                   │
│ • Mark (Guest) — Read Only (Expires in 7 days)              │
│ [+ Add Person / Delegate Capability]                        │
├─────────────────────────────────────────────────────────────┤
│ 💾 REPLICATION & PHYSICAL RESIDENCY                         │
│ 🟢 Safe on 3 Replicas                                       │
│  ├─ 📱 Pixel 9 Pro (Local Primary — Verified)               │
│  ├─ 💻 Studio Desktop (LAN Mesh — Verified SMT Root)        │
│  └─ 🏡 Family Home Node (LAN Mesh — Verified SMT Root)      │
├─────────────────────────────────────────────────────────────┤
│ 🎚️ [EXPERIENCE SLIDER = OPERATOR] DIAGNOSTIC DATA           │
│ SMT Leaf: 0x882f… | CAS Root: 0x110e…                       │
│ FastCDC Chunks: 4 (Dedup Ratio: 1.0x) | WAL Frame: #1042    │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. Navigation Density & Spatial Continuity

1. **Persistent Global Frame `[Observed]`:**
   - **Top Navigation Bar:** Product Title ("NEX"), Active Space Switcher, Identity Badge, Sync Status Pill, and the Global Experience Slider.
   - **Left Persistent Sidebar:** Universal Lens switchers (Home, Photos, Media, Maps, Drive, People, Devices, Family, Network, Settings).
   - **Central Canvas:** Lens viewport rendering the active projection.
   - **Right Contextual Inspector:** Slide-out drawer displaying object provenance, capability delegation, and physical replication state.
2. **Context Retention Across Lens Transitions `[NEX-specific]`:** If an object (e.g. `Integrated Family Photo.jpg`) is selected in the Family Space and the user switches from **Photos** to **Maps**, the selection remains active. Maps immediately pans to Lake Tahoe, and the Inspector continues displaying the photo's properties.
