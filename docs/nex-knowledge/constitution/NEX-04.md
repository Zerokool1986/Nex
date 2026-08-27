# NEX-04: Transport Abstraction Layer (TAL)

**Authority:** NEX Supreme Constitutional Law (Level 1)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-04_TRANSPORT_ABSTRACTION.md`  
**Status:** FROZEN & IMMUTABLE  

---

## 1. Constitutional Directives

`[DIRECT SOURCE FACT]`
1. **Transport Independence:**
   NEX Core is completely decoupled from physical transports. It treats transports as pluggable bidirectional frame conduits.
2. **Supported Conduits:**
   - **Loopback Socket / IPC:** High-speed local daemon communication.
   - **Direct TCP / TLS / QUIC:** High-throughput LAN and WAN direct connections.
   - **WebRTC DataChannels:** Browser-to-node and NAT-traversing peer connections.
   - **Reticulum Mesh:** Low-bandwidth, delay-tolerant, radio, and off-grid packet transport.

---

## 2. Implementation Grounding in Substrate

`[IMPLEMENTATION OBSERVATION]`
- In `nex-core/src/transport/adapter.rs`:
  - `TransportAdapter` trait defines methods `transport_tag()`, `mtu()`, `guarantee()`, `is_connected()`, `send()`, and `poll_incoming()`.
  - `TcpTransportAdapter` (tag `0x03`, 4MB MTU, `ReliableStream`) implements non-blocking direct TCP stream I/O.
  - `ReticulumNativeAdapter` (tag `0x01`, 500-byte link MTU, `UnreliableDatagram`) implements packet fragmentation via `FragmentationReassembler` and 16-byte destination hashing (`derive_reticulum_destination_hash`).
  - `MockQuicAdapter` (tag `0x02`, 64KB MTU, `ReliableStream`) provides QUIC channel abstraction.
- In `nex-core/src/transport/socket.rs`:
  - `LanTcpTransportServer` and `LanTcpTransportClient` provide concrete LAN peer synchronization over TCP sockets using magic `NXSK`.

---

## 3. Governance Scope

- **What NEX-04 Governs:**
  1. Strict transport agnosticism in `nex-core`.
  2. The pluggable conduit interface (`TransportAdapter`).
  3. Fragmentation and reassembly contracts for low-MTU networks (e.g., Reticulum mesh).
- **What NEX-04 Explicitly Does NOT Govern:**
  1. Payload state semantics or CRDT reconciliation (governed by `NEX-01`).
  2. Capability verification or token checking (governed by `NEX-03`).
  3. Application-level RPC method dispatching (governed by `nex-core/src/ipc/rpc.rs`).
