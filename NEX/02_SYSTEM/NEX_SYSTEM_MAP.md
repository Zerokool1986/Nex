# NEX System Map & Component Topology

```text
[ Client Hosts: Android / Desktop / Web ]
           │
  [ FFI / JNI / Loopback RPC ] (Gates R56, R57)
           │
  [ NexAppApi & Core Runtime ] (Gates R50, R51, R52, R58)
     ├── Object Store & SMT State Root
     ├── WAL Persistence & Snapshot Compaction
     ├── Capability Verifier & Delegation Engine
     └── Offline Outbox & Petname Directory
           │
  [ Extended Service Layer ]
     ├── Web Gateway & WebRTC Bridge (Gate R59)
     ├── Maps & Spatial Vector Tiles (Gate R60)
     ├── Groups, Family & Ratchets (Gate R61)
     ├── DHT Discovery & Vault Search (Gate R62)
     ├── Collaborative Resource Grid (Gate R63)
     └── Sandboxed WASM Compute Mesh (Gate R64)
```
