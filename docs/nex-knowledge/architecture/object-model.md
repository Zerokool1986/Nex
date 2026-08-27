# Universal Object Model Architecture

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-core/src/object/types.rs`, `nex-core/src/object/store.rs`, `NEX/02_SYSTEM/NEX_DATA_MODEL.md`  

---

## 1. Universal Object Schema (`NexObject`)

`[DIRECT SOURCE FACT]` & `[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/object/types.rs`, all stateful items in NEX are represented by the unified struct `NexObject`:

```rust
pub type ObjectID = [u8; 32];
pub type NamespaceID = [u8; 32];

pub struct NexObject {
    pub object_id: ObjectID,
    pub object_type: ObjectType,
    pub namespace: NamespaceID,
    pub owner_actor_id: ActorID,
    pub schema_version: u16,
    pub created_epoch: u64,
    pub created_lamport: u64,
    pub metadata: BTreeMap<String, String>,
    pub payload_bytes: Vec<u8>,
    pub tombstoned: bool,
}
```

### Enumerated Object Types (`ObjectType`)
```rust
#[repr(u16)]
pub enum ObjectType {
    DriveInode   = 0x0101,
    DriveFolder  = 0x0102,
    PhotoMedia   = 0x0201,
    PhotoAlbum   = 0x0202,
    ChatChannel  = 0x0301,
    ChatMessage  = 0x0302,
    ChatReceipt  = 0x0303,
    Community    = 0x0401,
    MemberRole   = 0x0402,
    VaultItem    = 0x0501,
    BackupIndex  = 0x0601,
    Synthetic(u16),
}
```

---

## 2. Deterministic Identifier & Content Addressing

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/api/mod.rs` (`NexCoreRuntime::create_object`), `ObjectID` is computed deterministically:
```rust
let mut hasher = Sha256::new();
hasher.update(b"NEX/OBJECT_ID/v1");
hasher.update(&namespace);
hasher.update(&self.actor_id);
hasher.update(&(self.state_node.current_lamport + 1).to_le_bytes());
hasher.update(&payload);
let object_id: [u8; 32] = hasher.finalize().into();
```

---

## 3. The Universal Object Store (`NexObjectStore`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/object/store.rs`:
```rust
pub struct NexObjectStore {
    pub objects: BTreeMap<ObjectID, NexObject>,
    pub namespace_index: BTreeMap<NamespaceID, Vec<ObjectID>>,
    pub owner_index: BTreeMap<ActorID, Vec<ObjectID>>,
}
```
- **Authoritative vs Derived:** `NexObjectStore.objects` contains the authoritative object entries. `namespace_index` and `owner_index` are derived secondary indices rebuilt upon insertion.
- **Tombstone Semantics:** Calling `tombstone(&object_id)` flags `tombstoned = true`. Deleted objects retain their causal history in the DAG and CRDT state while hiding payload content from application queries.

---

## 4. Universal Object Model vs NEX-01 / NEX-05 Boundary Analysis

`[INFERENCE]` & `[OPEN QUESTION]`
- **The Substrate Seam:** `NEX-01` governs the raw mathematical DAG and CRDT operations (`CrdtPayload::AddLWW { id, value }`, `CrdtPayload::Tombstone { id }`). `VirtualNode` operates strictly over raw 32-byte keys and byte buffers without awareness of `ObjectType` or `metadata` maps.
- **The Object Layer:** `NexObjectStore` and `NexAppApi` layer domain semantics (Drive, Photos, Chat, Vault) over the CRDT register.
- **Anti-Entropy Synthetic Fallback:** When anti-entropy sync receives a mutation for an object not previously recorded in local `NexObjectStore`, `AntiEntropyEngine` instantiates a fallback object with `ObjectType::Synthetic(1)` and empty metadata. Full metadata sync occurs through dedicated object state payloads.
