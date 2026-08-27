# NEX Architectural Mental Model

## How NEX Reasons About the World

```text
[ Physical World: Phones, Laptops, Desktops, Servers, Radios, Routers ]
                               │
                [ 1. Transport Abstraction Layer ]
           (WebRTC / TCP / Reticulum / Bluetooth / Unix Socket)
                               │
                 [ 2. Sovereign Core Runtime ]
          (Ed25519 Identity / Append-Only WAL / SMT State DAG)
                               │
               [ 3. Platform Primitives Substrate ]
    (CAS Storage / Capabilities / Outbox / Petnames / Groups / DHT)
                               │
          [ 4. Collaborative Resource & Compute Mesh ]
        (Erasure Shards / PoR Challenges / WASM Fuel Engine)
                               │
                [ 5. Sovereign Application Suite ]
      (Drive / Photos / Chat / Communities / Vault / Maps / Web)
                               │
                [ 6. Universal UX & Host Shells ]
                 (Android / Desktop / Web Browser)
```
