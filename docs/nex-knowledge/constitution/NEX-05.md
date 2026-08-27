# NEX-05: Security & Adversarial Threat Model

**Authority:** NEX Supreme Constitutional Law (Level 1)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-05_SECURITY_THREAT_MODEL.md`  
**Status:** FROZEN & IMMUTABLE  

---

## 1. Constitutional Directives

`[DIRECT SOURCE FACT]`
- **Threat Boundaries:**
  1. **Untrusted Network:** All wire traffic is AEAD encrypted with ephemeral session keys. Eavesdropping, MITM tampering, and packet replaying are prevented by design.
  2. **Untrusted Storage Providers:** Remote peers storing shards only see encrypted erasure pieces and must pass periodic Proof-of-Retrievability (PoR) challenges.
  3. **Untrusted Workers:** WASM compute kernels run in strict fuel-metered sandboxes with deterministic result commitments:
     $$\text{Commitment} = \text{SHA256}(\text{"NEX/COMPUTE\_RESULT/v1"} \,\|\, \text{JobID} \,\|\, \text{Output} \,\|\, \text{Fuel})$$

---

## 2. Implementation Grounding in Substrate

`[IMPLEMENTATION OBSERVATION]`
- In `nex-core/src/resilience/`:
  - `preflight_shield.rs` verifies packet signatures and session counters before allocating memory.
  - `peer_jail.rs` tracks malformed/malicious behavior and enforces exponential timeout jailing.
  - `rate_limiter.rs` enforces token-bucket ingress rate limits per peer `ActorID`.
- In `nex-core/src/apps/erasure.rs` and `resources.rs`:
  - Reed-Solomon erasure coding splits data into $M$ data + $N$ parity shards.
  - PoR engine generates deterministic challenge challenges and verifies HMAC responses.
- In `nex-core/src/apps/compute.rs`:
  - Sandboxed WASM execution environment measures instruction fuel and commits deterministic output hashes.

---

## 3. Governance Scope

- **What NEX-05 Governs:**
  1. Network adversarial threat assumptions and zero-trust transport boundary.
  2. Remote peer zero-knowledge storage verification (PoR).
  3. Sandboxed untrusted compute boundaries and fuel-metered execution.
- **What NEX-05 Explicitly Does NOT Govern:**
  1. Internal object schemas or inode tree models (governed by the Universal Object Model).
  2. Local disk write durability (governed by `NEX-02`).
  3. UI presentation density (governed by `NEX-UX-01`).
