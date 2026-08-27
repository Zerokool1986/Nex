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
