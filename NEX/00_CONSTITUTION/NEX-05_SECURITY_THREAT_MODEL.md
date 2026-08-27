# NEX-05: Security & Adversarial Threat Model

## 1. Threat Boundaries
- **Untrusted Network:** All wire traffic is AEAD encrypted with ephemeral session keys. Eavesdropping, MITM tampering, and packet replaying are prevented by design.
- **Untrusted Storage Providers:** Remote peers storing shards only see encrypted erasure pieces and must pass periodic Proof-of-Retrievability (PoR) challenges.
- **Untrusted Workers:** WASM compute kernels run in strict fuel-metered sandboxes with deterministic result commitments.
