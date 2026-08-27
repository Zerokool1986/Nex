# Platform Realization Architecture: Desktop & Android

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-desktop/`, `android/`, `nex-core/src/ffi/`, `nex-core/src/runtime/`  

---

## 1. Desktop Host Architecture (`nex-desktop`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-desktop/`:
- **Windowing & Framework:** Pure Rust native GUI powered by `eframe` and `egui` (`crates/nex-desktop/src/main.rs`).
- **Data Initialization:** Launches embedded `NexNode` over local disk path (`d:\Nex\nex_desktop_data`) with an ephemeral Ed25519 signing key or persistent seed.
- **UI Surfaces & Lenses:**
  - `src/ui/home.rs`: Main Home Shell showing Space switcher, connected devices, and activity stream.
  - `src/ui/drive.rs`: Drive file browser with FastCDC CAS chunk ingestion and drag-and-drop file import.
  - `src/ui/photos.rs`: Photos grid lens displaying thumbnail previews and metadata badges.
  - `src/ui/people.rs`: Contacts view with QR SAS pairing and petname confidence scores.
  - `src/ui/inspector.rs`: Universal Object Inspector sliding panel displaying live SMT roots, Lamport clocks, and capability proofs.
  - `src/ui/settings.rs`: Interactive 4-step Experience Slider (Simple, Standard, Advanced, Expert).

---

## 2. Android Mobile Host Architecture (`android/`)

`[IMPLEMENTATION OBSERVATION]`
In `android/app/src/main/java/app/nex/client/`:
- **Native Loading:** `NexClientApp.kt` loads `libnex_core.so` via JNI on application initialization.
- **Hardware KeyStore Inspection:** `NexKeystoreProvider.kt` inspects actual `KeyInfo` metadata:
  - Calls `keyInfo.isInsideSecureHardware` and checks `KeyProperties.SECURITY_LEVEL_STRONGBOX` or `TRUSTED_ENVIRONMENT`.
  - Strictly follows truthful evidence discipline: reports `"Hardware backing: NOT VERIFIED (Software Keyring / Ed25519)"` when StrongBox/TEE cannot be empirically proven.
- **LAN Socket Service:** `NexSocketSyncService.kt` launches a background service connecting to desktop nodes over local TCP sockets (`LanTcpTransportServer`) for real-time SMT anti-entropy replication.
- **Camera Ingestion:** `NexCameraManager.kt` bridges CameraX photo capture into native CAS storage chunks.

---

## 3. C ABI v1 & JNI Direct Buffer Bridge

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/ffi/`:
- `c_abi.rs`: Exported `extern "C"` functions (`nex_abi_version`, `nex_runtime_init`, `nex_runtime_destroy`, `nex_dispatch_rpc`) operating over thread-safe handle references (`NexHandle`).
- `jni_bridge.rs`: JNI bindings passing DirectByteBuffers without intermediate copying across the Rust/JVM boundary.
