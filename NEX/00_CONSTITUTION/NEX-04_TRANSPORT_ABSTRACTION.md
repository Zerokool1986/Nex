# NEX-04: Transport Abstraction Layer (TAL)

## 1. Transport Independence
NEX Core is completely decoupled from physical transports. It treats transports as pluggable bidirectional frame conduits.

## 2. Supported Conduits
- **Loopback Socket / IPC:** High-speed local daemon communication.
- **Direct TCP / TLS / QUIC:** High-throughput LAN and WAN direct connections.
- **WebRTC DataChannels:** Browser-to-node and NAT-traversing peer connections.
- **Reticulum Mesh:** Low-bandwidth, delay-tolerant, radio, and off-grid packet transport.
