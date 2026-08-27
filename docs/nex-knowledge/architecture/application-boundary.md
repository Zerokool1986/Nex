# Application Boundary & Universal Platform Primitives

**Status:** Authoritative Architectural Mapping  
**Source Locations:** `nex-core/src/api/mod.rs`, `nex-core/src/ipc/rpc.rs`, `NEX/00_CONSTITUTION/NEX-00_MASTER_VISION.md`  

---

## 1. The `NexAppApi` Trait Contract

`[DIRECT SOURCE FACT]` & `[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/api/mod.rs`, all applications (Drive, Photos, Chat, Communities, Vault, Backup, Maps, Web) interact with the core runtime through the unified trait `NexAppApi`:

```rust
pub trait NexAppApi {
    fn create_object(
        &mut self,
        namespace: NamespaceID,
        object_type: ObjectType,
        metadata: BTreeMap<String, String>,
        payload: Vec<u8>,
    ) -> Result<ObjectID, CoreRuntimeError>;

    fn mutate_object(
        &mut self,
        object_id: ObjectID,
        new_metadata: Option<BTreeMap<String, String>>,
        new_payload: Option<Vec<u8>>,
        proof: Option<CapabilityProof>,
    ) -> Result<[u8; 32], CoreRuntimeError>;

    fn read_object(&self, object_id: &ObjectID) -> Result<NexObject, CoreRuntimeError>;

    fn delete_object(
        &mut self,
        object_id: ObjectID,
        proof: Option<CapabilityProof>,
    ) -> Result<[u8; 32], CoreRuntimeError>;

    fn delegate_capability(
        &mut self,
        subject: ActorID,
        namespace: NamespaceID,
        object_id: Option<ObjectID>,
        allowed_ops: u32,
        valid_epochs: (u64, u64),
    ) -> Result<CapabilityProof, CoreRuntimeError>;

    fn sync_now(&mut self) -> Result<Checkpoint, CoreRuntimeError>;
}
```

---

## 2. Universal Platform Primitives

`[DIRECT SOURCE FACT]`
Under ADR-004 and the Universal Platform Rule, applications MUST NOT introduce parallel identity or sync stacks. Every application feature is realized by composing the 8 platform primitives:

1. **Object CRUD:** `create_object`, `read_object`, `mutate_object`, `delete_object`.
2. **Capability Delegation:** `delegate_capability` for secure multi-actor sharing.
3. **Causal SMT Synchronization:** `sync_now` and background anti-entropy.
4. **Content-Addressed Binary Storage:** CAS chunking and deduplication for large blobs.
5. **Petname Directory:** Transitive trust naming without centralized registries.
6. **Namespace Separation:** Cryptographic partition of data domains.
7. **Offline Durability:** Local-first queueing in `wal.log` and `OfflineOutbox`.
8. **Provenanced Activity Audit:** Universal history tracing via the DAG causal chain.

---

## 3. JSON-RPC 2.0 Daemon Dispatcher (`NexRpcDispatcher`)

`[IMPLEMENTATION OBSERVATION]`
In `nex-core/src/ipc/rpc.rs`:
- Provides daemon IPC over UNIX domain sockets and Windows named pipes.
- Supported methods include:
  - `nex_getStatus`: Returns operational state, actor ID, and SMT root.
  - `nex_createObject`: Dispatches `create_object` via `NexAppApi`.
  - `nex_readObject`: Queries `NexObjectStore` by `ObjectID`.
  - `nex_mutateObject`: Mutates payload/metadata with optional `CapabilityProof`.
  - `nex_deleteObject`: Appends tombstone mutation.
  - `nex_syncNow`: Triggers checkpointing and anti-entropy pass.
