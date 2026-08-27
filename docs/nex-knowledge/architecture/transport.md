# Transport Abstraction Layer & Network Mechanics

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-core/src/transport/`, `NEX/00_CONSTITUTION/NEX-04_TRANSPORT_ABSTRACTION.md`  

---

## 1. The `TransportAdapter` Interface

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/transport/adapter.rs`:
```rust
pub trait TransportAdapter: Send + Sync {
    fn transport_tag(&self) -> u16;
    fn mtu(&self) -> usize;
    fn guarantee(&self) -> TransportGuarantee;
    fn is_connected(&self) -> bool;
    fn send(&mut self, destination: &[u8], payload: &[u8]) -> Result<(), TransportError>;
    fn poll_incoming(&mut self) -> Option<TransportPacket>;
}
```

### Transport Guarantees:
- `ReliableStream`: In-order byte stream delivery (TCP, QUIC, UDS).
- `UnreliableDatagram`: Best-effort packet delivery (Reticulum, UDP).

---

## 2. Concrete Transport Implementations

| Adapter | Tag | MTU | Guarantee | Implementation File | Characteristics |
|---|---|---|---|---|---|
| **Reticulum Mesh** | `0x01` | 500 B (Link) / 64 KB (Logical) | `UnreliableDatagram` | `src/transport/adapter.rs` | 16-byte destination hash routing (`NEX/RNS_DEST/v1`), packet chunking & reassembly. |
| **QUIC Stream** | `0x02` | 64 KB | `ReliableStream` | `src/transport/adapter.rs` | Low-latency stream conduit. |
| **TCP/IP Direct** | `0x03` | 4 MB | `ReliableStream` | `src/transport/adapter.rs` | Non-blocking multi-peer socket multiplexing. |
| **LAN SMT Socket** | `NXSK` | Dynamic | `ReliableStream` | `src/transport/socket.rs` | Dedicated peer discovery and anti-entropy SMT sync server/client. |

---

## 3. Reticulum Integration & Low-MTU Fragmentation

`[IMPLEMENTATION OBSERVATION]`
In `ReticulumNativeAdapter` and `FragmentationReassembler` (`src/transport/fragmentation.rs`):
- Destination Hash Derivation:
  $$\text{DestHash} = \text{SHA256}(\text{"NEX/RNS\_DEST/v1"} \,\|\, \text{ActorID})[0..16]$$
- Frame Encoding: Embeds payload in 13-byte link header (`NX` magic).
- Fragmentation: Chunks wire frames larger than physical link MTU (500 bytes) with 32-byte message IDs, chunk indices, and total chunk counts.
- Reassembly: Collects chunks until the complete frame arrives or epoch timeout expires.

---

## 4. Multi-Transport Dispatcher (`MultiTransportDispatcher`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/transport/dispatcher.rs`:
- Dynamically routes outgoing packets across available adapters based on destination reachability, MTU requirements, and network cost metrics.
- Prioritizes high-throughput local LAN/TCP connections for bulk CAS transfers while preserving Reticulum/mesh channels for low-bandwidth discovery and causal heads sync.
