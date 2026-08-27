# NEX-03: Self-Sovereign Identity & Trust

**Authority:** NEX Supreme Constitutional Law (Level 1)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-03_IDENTITY_TRUST.md`  
**Status:** FROZEN & IMMUTABLE  

---

## 1. Constitutional Directives

`[DIRECT SOURCE FACT]`
1. **Actor IDs:**
   An `ActorID` is the cryptographic derivation of a public key:
   $$\text{ActorID} = \text{SHA256}(\text{"NEX/ACTOR\_ID/v1"} \,\|\, \text{KeyType} \,\|\, \text{PublicKeyBytes})$$
2. **Web of Trust & Petnames:**
   Global naming authorities (DNS, ICANN, centralized handles) are rejected. Names are local petnames resolved transitively through the user's Web of Trust with exponential score dampening:
   $$\text{Score} = \text{Score}_A \times \text{Score}_B \times 0.5$$

---

## 2. Implementation Grounding in Substrate

`[IMPLEMENTATION OBSERVATION]`
- In `nex-core/src/identity/verifier.rs`:
  - `derive_actor_id()` implements the exact prefix `NEX/ACTOR_ID/v1` over Ed25519 (tag `1`) or Secp256k1 (tag `2`) public keys.
- In `nex-core/src/identity/types.rs`:
  - `ActorID` is typed as `[u8; 32]`.
  - `DeviceCertificate` binds a `device_actor_id` to a `master_actor_id` over an epoch validity window `(not_before_epoch, expires_at_epoch)` signed by the master key.
  - `CapabilityToken` and `CapabilityProof` implement capability-based authorization with cryptographic signature chains and depth counters.
  - `RevocationEpochFence` establishes cryptographic revocation fences signed by an issuer.
- In `nex-core/src/identity/recovery/shamir.rs` and `ceremony.rs`:
  - Social recovery utilizes Shamir secret sharing over GF(256) with guardian quorum thresholds and timelocks.

---

## 3. Governance Scope

- **What NEX-03 Governs:**
  1. Cryptographic derivation and uniqueness of ActorIDs.
  2. Sub-device key delegation via `DeviceCertificate`.
  3. Petname directory resolution and Web of Trust confidence scoring.
  4. Capability token hashing, attenuation, and revocation fences.
- **What NEX-03 Explicitly Does NOT Govern:**
  1. Network packet routing or physical socket connections (governed by `NEX-04`).
  2. SMT state root computation (governed by `NEX-01`).
  3. UI presentation density (governed by `NEX-UX-01`).
