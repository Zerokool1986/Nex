# NEX Architectural Decision Records (ADRs)

## ADR-001: Rejection of Centralized Signaling Servers
- **Status:** SEALED & FROZEN
- **Context:** P2P connection establishment.
- **Decision:** Use DHT-based peer rendezvous and local multicast instead of mandatory central STUN/TURN/Signaling servers.
- **Rejected:** Cloud-hosted central rendezvous registries.

## ADR-002: Rejection of "Reticulum Everywhere" as Universal Substrate
- **Status:** SEALED & FROZEN
- **Context:** High-throughput streaming vs low-bandwidth mesh.
- **Decision:** Implement a Transport Abstraction Layer. Reticulum is a pluggable transport for off-grid mesh, while WebRTC and TCP handle high-throughput CAS streaming.
- **Rejected:** Forcing all video, photo, and file traffic over Reticulum packet streams.

## ADR-003: Rejection of Blockchain Consensus
- **Status:** SEALED & FROZEN
- **Context:** Multi-device state replication.
- **Decision:** Use state-based CRDTs, SMT state roots, and bilateral credit accounting instead of global blockchain consensus.
- **Rejected:** Ethereum/Solana style global ledgers and gas fees.

## ADR-004: Universal Platform Primitives for All Applications
- **Status:** SEALED & FROZEN
- **Context:** Development of Drive, Chat, Photos, Maps, Web, Vault.
- **Decision:** All applications must consume the 8 Universal Platform Primitives from `NexAppApi`.
- **Rejected:** Independent, bespoke silos with parallel identity and sync implementations.
