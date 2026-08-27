---
description: NEX Non-Negotiables and Rejected Antipatterns
always_apply: true
---

# NEX Architectural Invariants & Non-Negotiables

## 1. What NEX Is
- A unified personal sovereign computing platform.
- A local-first, offline-capable, peer-to-peer replicated data environment.
- A capability-secured resource and execution mesh.
- A consumer of common platform services across all user-facing applications.

## 2. What NEX Is Deliberately NOT (Rejected Antipatterns)
- **NEX is NOT a Blockchain:** No global proof-of-work, no global proof-of-stake, no global total transaction ordering, no speculative tokens, no gas auctions.
- **NEX is NOT "Reticulum Everywhere":** Reticulum is one supported transport for discovery and low-bandwidth mesh; bulk data and streaming use WebRTC, QUIC, TCP, or LAN direct.
- **NEX is NOT a Cloud Storage Wrapper:** NEX does not depend on AWS, GCP, Azure, or centralized S3 buckets. Storage is self-hosted, peer-replicated, or collaborative.
- **NEX is NOT a Collection of Disconnected Apps:** Drive, Chat, Photos, Vault, Maps, and Web are clients of the single unified NEX platform. They NEVER invent separate identity, permission, or sync engines.
- **NEX is NOT Dependent on LoRa:** NEX runs over standard Wi-Fi, Ethernet, Bluetooth, Loopback, Cellular, and mesh links alike.
- **NEX does NOT Sacrifice Usability for Ideology:** Interfaces must be fast, responsive, modern, and accessible to non-technical users.
