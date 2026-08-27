# NEX Product Families, Experiences & Persona Matrix

## 1. The 15 Unified NEX Experiences

To prevent fragmented app sprawl and cognitive overload, NEX organizes its user-facing capabilities into **15 Coherent Product Families**. The underlying sovereign substrate handles the complexity, while the user experiences seamless, integrated products.

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                                 NEX HOME                                    │
│                    (Your Personal Sovereign Environment)                    │
├──────────────────────────────────────┬──────────────────────────────────────┤
│  💬 NEX COMMS                        │  📁 NEX DRIVE                        │
│     Chat • Voice • Video             │     Files • Sync • Sharing           │
│     Mail • People / Contacts         │     Offline Access • CAS Deduplication│
├──────────────────────────────────────┼──────────────────────────────────────┤
│  📷 NEX PHOTOS                       │  🗺 NEX MAPS                         │
│     Photos • Memories • Albums       │     Offline Vector Maps • Navigation │
│     Video Org • Local ML Search      │     Private Track Logs • Waypoints   │
├──────────────────────────────────────┼──────────────────────────────────────┤
│  🎬 NEX MEDIA                        │  📝 NEX PRODUCTIVITY                 │
│     Personal Media Server • Music    │     Tasks • Calendar • Notes         │
│     Movies • P2P Device Streaming    │     Office • Projects                │
├──────────────────────────────────────┼──────────────────────────────────────┤
│  🤖 NEX AI                           │  ⚙ NEX AUTOMATION (IoT)             │
│     Local / Private Intelligence     │     Event-Driven Workflows           │
│     Capability-Secured Assistant     │     Smart Home • Device Sensors      │
├──────────────────────────────────────┼──────────────────────────────────────┤
│  🔐 NEX VAULT                        │  💾 NEX BACKUP                       │
│     Zero-Knowledge Passwords/Secrets │     Continuous SMT Vault Protection  │
│     Private Documents & Keys         │     Disaster Recovery • Multi-Device │
├──────────────────────────────────────┼──────────────────────────────────────┤
│  🌐 NEX WEB                          │  ◇ NEX APPLICATIONS                  │
│     Sovereign Web Gateway (127.0.0.1)│     App Discovery • Permissions      │
│     WebRTC Bridge • Sandboxed Apps   │     Sandboxed Runtime • Updates      │
├──────────────────────────────────────┼──────────────────────────────────────┤
│  💳 NEX ECONOMICS                    │  🛠 NEX DEVELOPER                    │
│     Payment-Rail Agnostic Commerce   │     First-Party Tooling & SDKs       │
│     Resource Contribution Markets    │     Local Simulators • Manifests     │
└──────────────────────────────────────┴──────────────────────────────────────┘
```

---

## 2. Granular Breakdown of the 15 Product Families

### 1. NEX Home
- **User Purpose:** The central dashboard and launcher for the user's sovereign computing life.
- **Experience:** Persona-tailored home screen, global notification feed, device health status, and cross-application search.

### 2. NEX Drive
- **User Purpose:** Complete dominion over personal files and document hierarchies.
- **Experience:** Local-first folder structures, instant CAS deduplication, peer-to-peer sharing, offline availability, and family shared storage folders.

### 3. NEX Photos
- **User Purpose:** Protecting and organizing a lifetime of visual memories.
- **Experience:** High-performance media grid, smart album clustering, on-device ML visual search, EXIF indexing, and full-resolution family sharing without cloud compression.

### 4. NEX Backup
- **User Purpose:** Total disaster recovery and continuous data protection.
- **Experience:** Automated background SMT snapshotting, cross-device replication, version history time-machine, and zero-cloud bare-metal restore.

### 5. NEX Vault
- **User Purpose:** Zero-knowledge protection for sensitive credentials and confidential records.
- **Experience:** Hardware keystore-backed credential manager, private notes, identity seed storage, and OS autofill provider integration.

### 6. NEX Media
- **User Purpose:** Decentralized home media streaming.
- **Experience:** Music and video streaming server, offline library caching, on-the-fly transcoding, and casting to local screens without third-party tracking.

### 7. NEX Comms
- **User Purpose:** All human communication under sovereign cryptographic control.
- **Unified Components:**
  - **Nex Chat:** Asynchronous append-only E2EE messaging spool with forward-secrecy epoch ratchets.
  - **Nex Voice:** P2P low-latency real-time voice calls over WebRTC/TAL.
  - **Nex Video:** High-definition video conferencing without centralized conference bridges.
  - **Nex Mail:** Sovereign asynchronous mail bridge and local inbox.
  - **Nex People / Contacts:** Cryptographic Web-of-Trust address book and relationship graph.

### 8. NEX Maps
- **User Purpose:** Private, decentralized geospatial navigation and track logging.
- **Experience:** 100% offline vector tile pyramid rendering, private GPS track recording, shared family waypoints, and geo-fenced community places without location tracking.

### 9. NEX Productivity
- **User Purpose:** Integrated personal and collaborative work suite.
- **Unified Components:**
  - **Nex Tasks:** Sovereign task lists and checklist management.
  - **Nex Calendar:** Encrypted scheduling, recurring events, and multi-peer availability.
  - **Nex Notes:** Local-first rich markdown notes with SMT version history.
  - **Nex Office:** Local-first document, spreadsheet, and slide editing.
  - **Nex Projects:** Kanban boards, milestones, and issue tracking.

### 10. NEX Web
- **User Purpose:** Bridging sovereign NEX territory with the conventional Internet.
- **Experience:** Local HTTP gateway (`127.0.0.1:8080`), WebRTC NASP bridge for web browsers, decentralized publishing, and secure WebApp sandboxes.

### 11. NEX AI
- **User Purpose:** Private, on-device personal intelligence.
- **Experience:** Local LLM/embedding inference, document indexing, conversational assistance, and agentic workflows operating strictly under the capability token model (zero ambient authority).

### 12. NEX Automation (IoT)
- **User Purpose:** Local-first smart device orchestration and event-driven automation.
- **Experience:** Local sensor monitoring, smart home controls, automated rule triggers, and physical mesh device integrations.

### 13. NEX Applications
- **User Purpose:** The application ecosystem and runtime manager.
- **Experience:** App discovery, sandboxed installation, fine-grained capability approval prompts, subscription management, and deterministic updates.

### 14. NEX Developer
- **User Purpose:** The standalone developer environment and tooling pillar.
- **Experience:** Multi-language `libnex` SDKs (Rust, Kotlin, TypeScript, C), local multi-node simulation testbeds, manifest builders, and WASM compute compiler toolchains.

### 15. NEX Economics
- **User Purpose:** Sovereign, payment-rail agnostic commerce and resource contribution.
- **Experience:** Connecting user-chosen payment providers (cards, bank, crypto), managing subscriptions, P2P micropayments, and earning credits by contributing storage/compute.

---

## 3. Persona Mapping Across Product Families

```text
+====================================================================================================+
| PRODUCT FAMILY      | CONSUMER / FAMILY   | PROSUMER / HOMELAB  | OFF-GRID / FIELD    | BUILDER    |
+====================================================================================================+
| NEX Home            | Simple dashboard    | Multi-node monitor  | Minimal telemetry   | Dev portal |
| NEX Drive           | Family photo/docs   | Multi-TB sync pool  | Offline caching     | Test assets|
| NEX Photos          | Auto-sync albums    | RAW storage / NAS   | Offline gallery     | Media API  |
| NEX Backup          | Automated restore   | Headless snapshots  | Sneakernet backups  | SMT dumps  |
| NEX Vault           | Simple passwords    | SSH keys & certs    | Tactical seed vault | App secrets|
| NEX Media           | Home TV streaming   | Transcode server    | Cached audio files  | Media SDK  |
| NEX Comms           | Family group chat   | Self-hosted relay   | LoRa packet radio   | Bot APIs   |
| NEX Maps            | Offline trip maps   | Custom map layers   | Tactical GPS trails | Spatial SDK|
| NEX Productivity    | Grocery & calendar  | Project management  | Offline field logs  | Schema APIs|
| NEX Web             | Local browser view  | Custom Web gateway  | Zero-cloud portal   | WASM shells|
| NEX AI              | Photo/Doc search    | Local GPU inference | Offline tiny-model  | Agent tools|
| NEX Automation      | Smart lights / lock | Node health scripts | Sensor data logging | Webhooks   |
| NEX Applications    | 1-click install     | Daemon management   | Sideload packages   | App Publish|
| NEX Developer       | —                   | Custom plugins      | Mesh debug tools    | Full SDK   |
| NEX Economics       | Simple subscription | Storage provider    | Credit trading      | App billing|
+====================================================================================================+
```
