# Identity, Trust & Capability Architecture

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-core/src/identity/`, `NEX/00_CONSTITUTION/NEX-03_IDENTITY_TRUST.md`, `NEX/02_SYSTEM/NEX_IDENTITY_MODEL.md`, `NEX/02_SYSTEM/NEX_CAPABILITY_MODEL.md`  

---

## 1. ActorID & Cryptographic Key Derivation

`[DIRECT SOURCE FACT]` & `[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/identity/verifier.rs` (`derive_actor_id`):
```rust
pub fn derive_actor_id(key_type: KeyType, pubkey_bytes: &[u8]) -> ActorID {
    let mut hasher = Sha256::new();
    hasher.update(b"NEX/ACTOR_ID/v1");
    hasher.update(&[key_type as u8]);
    hasher.update(pubkey_bytes);
    hasher.finalize().into()
}
```
- Supported Key Types: `Ed25519` (`1`), `Secp256k1` (`2`).

---

## 2. Device Certificates & Sub-Key Delegation

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/identity/types.rs`:
```rust
pub struct DeviceCertificate {
    pub master_actor_id: ActorID,
    pub device_actor_id: ActorID,
    pub not_before_epoch: u64,
    pub expires_at_epoch: u64,
    pub master_pubkey: Option<Vec<u8>>,
    pub signature: Vec<u8>,
}
```
- Devices run dedicated sub-keys authorized via a `DeviceCertificate` signed by the master identity.

---

## 3. Capability-Based Authorization Tokens (`CapabilityToken`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/identity/types.rs`:
```rust
pub struct CapabilityToken {
    pub issuer: ActorID,
    pub subject: ActorID,
    pub namespace: NamespaceID,
    pub object_id: Option<ObjectID>,
    pub allowed_operations: u32,
    pub delegation_depth: u8,
    pub not_before_epoch: u64,
    pub expires_at_epoch: u64,
    pub parent_token_hash: Option<[u8; 32]>,
}

pub struct CapabilityProof {
    pub token: CapabilityToken,
    pub issuer_pubkey: Option<Vec<u8>>,
    pub parent_proof: Option<Box<CapabilityProof>>,
    pub signature: Vec<u8>,
}
```

### Operation Bitmask Flags:
- `OP_REGISTER_LWW`: `0x01`
- `OP_SET_ADD`: `0x02`
- `OP_SET_REMOVE`: `0x04`
- `OP_SEQUENCE_INSERT`: `0x08`
- `OP_OBJECT_TOMBSTONE`: `0x10`
- `OP_READ`: `0x01`
- `OP_WRITE`: `0x02`
- `OP_ALL`: `0x1F`

---

## 4. Capability Chain Verification (`verify_capability_chain`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/identity/verifier.rs`:
Verification strictly enforces:
1. **Signature Validity:** Ed25519 signature over canonical token hash `SHA256("NEX/CAPABILITY_TOKEN/v1" || canonical_bytes)`.
2. **Epoch Window:** $\text{not\_before} \le \text{current\_epoch} \le \text{expires\_at}$.
3. **Revocation Check:** Checks token hash and issuer against active `RevocationEpochFence` entries.
4. **Operation Subset:** Requested bitmask must be a subset of `allowed_operations`.
5. **Attenuation Invariant:** Child tokens cannot grant permissions, namespaces, or epoch lifetimes broader than their parent proof.
6. **Delegation Depth:** `delegation_depth` strictly decrements at each delegation level until reaching `0`.

---

## 5. Social Recovery via Shamir Threshold Scheme

`[IMPL][TEST][UI]`
In `nex-core/src/identity/recovery/`:
- Splits 32-byte master seed into $N$ guardian shares over Galois Field $GF(2^8)$ with threshold $K$ (`shamir.rs`).
- Social recovery ceremony collects $K$ valid shares and enforces anti-hijack time-lock (`ceremony.rs`).
- `DeviceRecoveryWorkflow` reauthorizes replacement devices and registers lost devices into CRL without changing root `ActorID` (`device_recovery.rs`, `recovery.rs`).

---

## 6. Petname Directory & Web-of-Trust Evidence Status

### `[IMPL][TEST]` Flat Petname Directory
In `nex-core/src/apps/platform.rs`:
- Provides local, subjective petname alias mapping `String -> ActorID`.
- Direct resolution for user-assigned contact names.

### `[SPEC — NOT IMPLEMENTED]` Transitive Trust & Score Dampening
- Transitive Web-of-Trust graph traversal with multiplicative score dampening ($\text{Score}_A \times \text{Score}_B \times 0.5$) is a theoretical specification and is **NOT implemented** in code.
- Sybil resistance currently relies strictly on local invitation gating, out-of-band SAS pairing, and transport rate-limiting/jailing.
