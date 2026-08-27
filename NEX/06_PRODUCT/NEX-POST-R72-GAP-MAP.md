# NEX POST-R72 PRODUCTIZATION GAP MAP & EVIDENCE AUDIT

**Authority:** Chris / NEX Architecture  
**Baseline:** NEX Cognitive Architecture Package v3.5  
**Current Master Substrate Baseline:** **606 / 606 passing tests across 101 suites**  
**Constitution:** `NEX-00..05` + `NEX-UX-01` (FROZEN & IMMUTABLE)  
**Wire & Persistence Contracts:** `NEX/WIRE/v1`, `NEX/WAL/v1`, C ABI v1 (FROZEN & IMMUTABLE)  
**Status:** **AUTHORITATIVE PRODUCTIZATION LEDGER**  

---

## 1. Product Evidence Ladder Framework (L0 – L9)

Every capability in the NEX ecosystem is formally classified along the **10-level evidence ladder**:

```text
[L0] Architectural Definition      : Formally specified in constitutional ADRs & gate contracts.
[L1] Rust Substrate Realization    : Pure Rust data types, state machines, and mathematical logic.
[L2] Integration Test Verified     : Automated multi-node integration test passing in headless cargo harness.
[L3] Product-Model Realization     : High-fidelity ViewModels & Controllers mapped to canonical state.
[L4] Actual UI Realization         : Native GUI/TUI/Compose/Slint/Iced/Egui rendered on screen.
[L5] Real-Device Execution         : Physical execution on actual Android/Linux/macOS/Windows hardware.
[L6] Offline / Chaos Execution     : Physical disconnect, packet dropping, and process kill verified.
[L7] Cross-Device Physical Sync    : Physical wireless (WiFi Direct / Bluetooth / LAN) sync between 2 real hosts.
[L8] Human Usability Validation    : A non-technical human successfully performs everyday workflows unaided.
[L9] Production Hardening          : Long-running stress, automated upgrades, telemetry-free crash dumps.
```

---

## 2. Comprehensive Inventory of Current R72 Product Surfaces

| Surface / Capability | Current Evidence Level | Canonical State Backing | Host Implementation Status | Critical Missing Gaps |
|---|---|---|---|---|
| **NEX Home Surface** | **L3 (Product-Model)** | 🟢 100% Real (`NexNode`, DAG, SMT) | TUI/CLI (`src/cli/`), Headless ViewModel | Native Android Compose UI & Desktop GUI shell |
| **Spaces Selector** | **L3 (Product-Model)** | 🟢 100% Real (`NamespaceID` query filter) | TUI/CLI, Headless Controller | Visual Space switcher bar in native UI |
| **Photos Lens** | **L3 (Product-Model)** | 🟢 100% Real (`PhotoMedia` Inodes, CAS) | Rust headless lens (`HumanExperienceEngine`) | Native Image thumbnail rendering & grid layout |
| **Drive Lens** | **L3 (Product-Model)** | 🟢 100% Real (`DriveInode`, FastCDC CAS) | Rust headless lens (`DriveScreenViewModel`) | File tree browser & OS drag-and-drop handler |
| **Universal Object Inspector**| **L3 (Product-Model)** | 🟢 100% Real (Provenance, DAG, SMT) | Rust headless inspector (`UniversalObjectInspector`) | Interactive sliding drawer / inspector panel UI |
| **Person Panel (Amy)** | **L3 (Product-Model)** | 🟢 100% Real (`ActorID`, QR SAS, WoT) | Rust headless controller (`PersonPanelController`) | Native contact card, SAS QR scanner dialog |
| **Device Panel** | **L3 (Product-Model)** | 🟢 100% Real (Certs, TEE KeyStore, Quotas) | Rust headless controller (`DevicePanelController`) | Native paired device list & revocation modal |
| **Settings & Slider** | **L3 (Product-Model)** | 🟢 100% Real (4-tier complexity tree) | Rust headless tree (`SettingsController`) | Interactive 4-step UI slider & toggle controls |
| **Offline Capture** | **L6 (Chaos Test)** | 🟢 100% Real (Append-only WAL, Outbox) | Rust integration harness (`r71_34`, `r72_4`) | Android WorkManager trigger on real phone |
| **Cross-Device Sync** | **L6 (Chaos Test)** | 🟢 100% Real (SMT Anti-Entropy, Batches) | Rust integration harness (`r71_17`, `r71_33`) | Real P2P mDNS/WiFi Direct socket daemon |
| **Truthful Status Badge** | **L3 (Product-Model)** | 🟢 100% Real (Zero false "Synced" logic) | Rust engine (`ProductionRealityEngine`) | Native status bar indicator & notification icon |
| **Social Recovery** | **L2 (Integration)** | 🟢 100% Real (Shamir GF(256), Timelocks) | Rust crypto engine (`src/identity/recovery/`) | Step-by-step guardian ceremony wizard UI |

---

## 3. Detailed Gap Classification (10 Dimensions)

### Dimension 1: Surfaces that are Rust / Product-Model Only
- `src/product/home.rs`: Outputs `HomeScreenViewModel` struct; lacks native Android Jetpack Compose / Desktop GUI renderer.
- `src/product/inspector.rs`: Outputs `UniversalObjectInspector` struct; lacks visual sliding panel.
- `src/product/person.rs`: Outputs `PersonContextualSurface` struct; lacks native avatar and message input composer.
- `src/product/device.rs`: Outputs `DeviceContextualSurface` struct; lacks hardware pairing screen.
- `src/product/settings.rs`: Outputs `SettingsConsequenceTree` struct; lacks native settings navigation list.

### Dimension 2: Real Android Host Foundation vs. Gaps
- **Existing Real Code:** `src/ffi/jni_bridge.rs` (JNI direct buffer exchange, byte slicing), `src/runtime/mobile.rs` (Lifecycle states: Active, Backgrounded, Doze, DeepSleep).
- **Missing Gaps:** 
  1. Real Android Studio APK project wrapping `libnex.so`.
  2. Jetpack Compose UI rendering the ViewModels.
  3. CameraX capture bridge piping JPEG bytes directly into `SovereignProductSlice::mobile_capture_family_photo`.
  4. Android Keystore JNI binding for hardware-backed Ed25519 root signing.

### Dimension 3: Real Desktop Host Foundation vs. Gaps
- **Existing Real Code:** `src/runtime/desktop.rs` (Multi-window daemon, bearer token RPC), `src/ipc/` (UNIX domain socket & Windows Named Pipe IPC), `src/bin/nex.rs` (Native CLI).
- **Missing Gaps:**
  1. Desktop application binary with native Window Manager (e.g., Slint, Egui, or Tauri/Winit GUI).
  2. OS Keyring integration (Windows Credential Manager / macOS Keychain / Linux SecretService) via C ABI.
  3. Real filesystem watch folder for automatic Drive lens ingestion.

### Dimension 4: Simulated / Mocked vs. Substrate-Backed Capabilities
- **100% Canonical Substrate-Backed (NO MOCKS):**
  - Object creation, metadata indexing, FastCDC chunking, CAS SHA-256 store.
  - WAL append-only journaling, CRC32 verification, crash recovery.
  - SMT frontier calculation, Merkle DAG proof generation, anti-entropy batch reconciliation.
  - Ed25519 capability token verification, CRL revocation check, delegation chains.
- **Simulated in Test Harness (Requires Real Host I/O):**
  - Network transport in tests uses in-memory FIFO channels rather than physical TCP/UDP/QUIC sockets.
  - Hardware KeyStore in tests uses Dalek software signing rather than Android TEE / TPM 2.0 hardware.
  - Display rendering in tests consumes JSON/ViewModels rather than GPU pixels.

### Dimension 5: Real-Device Physical Validation Requirements
- **Pixel 9 Pro Physical Validation:**
  - Verify app pause during incoming phone call (Android lifecycle destruction) $\to$ resume without CAS corruption.
  - Verify photo capture in Airplane Mode $\to$ turn on Wi-Fi $\to$ automatic background sync to Desktop.
- **Desktop Physical Validation:**
  - Verify hard process termination (`kill -9`) $\to$ restart $\to$ lockfile recovery and SMT reconciliation.
  - Verify concurrent file edits in local folder $\to$ FastCDC deduplication.

### Dimension 6: Truthful State Presentation Audit
- **Strict Compliance:** The application model **never** reports "Synced" unless `target_frontier == local_frontier` and Merkle roots match.
- **Offline Integrity:** When offline, status badge explicitly says *"You're offline. Changes will sync automatically when you're connected."*
- **Replication Truth:** Displays *"2 Replicas"* only when proof of receipt is acknowledged by peer node.

---

## 4. Priority Ranked Gap Matrix (Impact × Dependency × Risk × Evidence Deficit)

| Rank | Gap Description | Subsystem | Human Impact | Tech Dependency | Risk | Evidence Deficit | Score (1–100) |
|---|---|---|---|---|---|---|---|
| **#1** | **Standalone Runnable Desktop GUI/TUI Shell** | Desktop / UI | High (User sees NEX) | Low (Uses existing RPC/ViewModel) | Low | High (L3 $\to$ L4) | **95** |
| **#2** | **Android APK Shell with Compose UI** | Mobile / UI | High (User holds NEX) | Medium (JNI $\to$ Compose) | Medium | High (L3 $\to$ L4) | **92** |
| **#3** | **Physical Socket Transport (Local LAN/WiFi P2P)** | Transport | Critical (Real sync) | Medium (Anti-entropy stream) | Medium | High (L2 $\to$ L7) | **90** |
| **#4** | **Camera Import & File Picker Binding** | Ingestion | High (Real photos) | Low (Feeds `capture_photo`) | Low | Medium (L3 $\to$ L5) | **85** |
| **#5** | **Native QR Code Scanner / SAS Pairing Dialog** | Identity | High (Easy pairing) | Medium (QR encoding exists) | Low | Medium (L2 $\to$ L5) | **82** |
| **#6** | **OS Keyring Integration (Windows/macOS/Linux)** | Security | High (Key safety) | Medium (Desktop RPC exists) | Medium | Medium (L2 $\to$ L5) | **78** |
| **#7** | **Interactive 4-Tier Experience Slider Controls** | UX | High (Cognitive calm) | Low (ViewModels exist) | Low | Low (L3 $\to$ L4) | **75** |
| **#8** | **Offline Outbox Background Sync Worker** | Background | High (Never lose data) | Medium (Outbox exists) | Low | Medium (L3 $\to$ L6) | **72** |

---

## 5. Immediate P0 Productization Milestone

### The Target: First Runnable Desktop & Mobile Interactive Experience

Transform the L3 product models into an **L4 / L5 interactive application** that executes the 20-step canonical journey on real hosts:

```text
[REAL HOST SHELL]
       │
       ▼
[NEX Home View] ──(Select Family Space)──▶ [Photos / Drive Lens]
       │                                           │
       │                                           ▼
[Person Panel: Amy] ◀──(Inspect Object)─── [Universal Inspector]
       │                                           │
       ▼                                           ▼
[Device Panel: Desktop] ──(Physical Mesh)──▶ [Real Anti-Entropy Sync]
```

### Mandatory Acceptance Criteria:
1. Running process connects to real `crates/nex-core` daemon.
2. User imports a real image from disk $\to$ sees thumbnail, size, and protection badge.
3. User opens Universal Inspector $\to$ sees live provenance and capability tokens.
4. User taps Amy $\to$ sees verified trust status and shared objects.
5. User toggles Experience Slider $\to$ sees presentation adapt without changing permissions.
6. All 606 existing authoritative tests remain 100% green.
