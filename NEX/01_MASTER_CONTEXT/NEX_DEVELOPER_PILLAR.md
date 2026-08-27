# NEX Developer: The Ecosystem Builder Pillar

## 1. Architectural Purpose

**NEX Developer** is not merely an app in the productivity suite; it is a foundational pillar bridging the **NEX Platform Core** to third-party developers, builders, and ecosystem applications.

```text
                  NEX PLATFORM CORE (crates/nex-core)
                               │
            ┌──────────────────┴──────────────────┐
            │                                     │
   FIRST-PARTY SUITE (Tier 1 & 2)          NEX DEVELOPER PILLAR
   (Drive, Photos, Chat, Vault...)                │
                                       ┌──────────┴──────────┐
                                       ▼                     ▼
                               [ `libnex` SDKs ]     [ Developer Tools ]
                               (Kotlin/TS/Rust/C)    (CLI, Mock, Testbed)
                                       │                     │
                                       └──────────┬──────────┘
                                                  ▼
                                     [ Third-Party Applications ]
                                     [ Custom Extensions & WASM ]
                                     [ AI / Agentic Development ]
```

---

## 2. Core Capabilities Provided by NEX Developer

1. **Multi-Language `libnex` SDKs:**
   - Native Rust crate bindings.
   - Kotlin / Java bindings via DirectByteBuffer JNI for Android.
   - TypeScript / JavaScript bindings for Web & Electron/Tauri.
   - C ABI v1 header bindings for iOS (Swift) and desktop C/C++ apps.
2. **Local Mock & Simulation Testbed:**
   - Multi-node in-memory mesh simulator for testing offline partitions, sync races, and capability delegation.
   - Local Golden Oracle capture tool for validating wire and state compatibility.
3. **Application Packaging & Manifest Builder:**
   - Tooling to package `NexApp` bundles with explicit namespace permission requests and CSP policies.
4. **WASM Compute Kernel Toolchain:**
   - SDK for compiling deterministic, fuel-metered WASM kernels for the Nex Compute Mesh.
5. **Agentic & AI-Assisted Integration:**
   - Machine-readable specifications and schemas allowing AI coding assistants (like Antigravity) to build sovereign applications conforming to NEX constitutional contracts.
