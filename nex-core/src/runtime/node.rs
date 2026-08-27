use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use ed25519_dalek::{SigningKey, Signer};
use crate::model::{ActorID, Mutation, MutationBody, Checkpoint, CrdtPayload};
use crate::hash::hash_mutation_body;
use crate::identity::types::{KeyType, CapabilityProof, CapabilityToken, OP_REGISTER_LWW, OP_WRITE, OP_OBJECT_TOMBSTONE};
use crate::identity::verifier::{derive_actor_id, verify_capability_chain, hash_capability_token};
use crate::object::types::{ObjectID, NamespaceID, ObjectType, NexObject};
use crate::storage::wal::WriteAheadLog;
use crate::storage::state_db::{StateDbEngine, StateSnapshotData};
use crate::apps::drive::CasChunkStore;
use crate::sync::node::VirtualNode;
use crate::sync::types::IngressDisposition;
use crate::runtime::production::NodeOperationalState;
use crate::api::{NexAppApi, CoreRuntimeError};
use crate::discovery::routing_table::RoutingTable;
use crate::transport::dispatcher::MultiTransportDispatcher;
use crate::transport::fragmentation::FragmentationReassembler;
use crate::resilience::rate_limiter::PeerRateLimiter;
use crate::resilience::peer_jail::PeerJail;
use crate::resilience::preflight_shield::PreFlightShield;
use crate::apps::drive::DriveEngine;
use crate::apps::chat::ChatEngine;
use crate::apps::community::CommunityEngine;

#[cfg(windows)]
extern "system" {
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
}

fn is_process_alive(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    #[cfg(unix)]
    {
        let proc_path = format!("/proc/{}", pid);
        if std::path::Path::new(&proc_path).exists() {
            return true;
        }
    }
    #[cfg(windows)]
    {
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const SYNCHRONIZE: u32 = 0x00100000;
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, 0, pid) };
        if !handle.is_null() {
            unsafe { CloseHandle(handle) };
            return true;
        }
    }
    false
}

pub struct IdentityEngine {
    pub signing_key: SigningKey,
    pub pubkey_bytes: Vec<u8>,
    pub actor_id: ActorID,
    pub active_revocations: BTreeMap<[u8; 32], u64>,
    pub blocklist: crate::identity::blocklist::PersonalBlocklist,
}

impl IdentityEngine {
    pub fn new(signing_key: SigningKey) -> Self {
        let pubkey_bytes = signing_key.verifying_key().to_bytes().to_vec();
        let actor_id = derive_actor_id(KeyType::Ed25519, &pubkey_bytes);
        Self {
            signing_key,
            pubkey_bytes,
            actor_id,
            active_revocations: BTreeMap::new(),
            blocklist: crate::identity::blocklist::PersonalBlocklist::new(),
        }
    }
}

pub struct StateEngine {
    pub state_node: VirtualNode,
    pub object_store: BTreeMap<ObjectID, NexObject>,
    pub current_epoch: u64,
    pub latest_mutation_id: Option<[u8; 32]>,
}

impl StateEngine {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            state_node: VirtualNode::new(node_id),
            object_store: BTreeMap::new(),
            current_epoch: 0,
            latest_mutation_id: None,
        }
    }
}

pub struct StorageEngine {
    pub data_dir: PathBuf,
    pub wal: Option<WriteAheadLog>,
    pub cas: CasChunkStore,
}

impl StorageEngine {
    pub fn new(data_dir: PathBuf, wal_path: Option<PathBuf>) -> Self {
        let wal = wal_path.and_then(|p| WriteAheadLog::open(p).ok());
        Self {
            data_dir,
            wal,
            cas: CasChunkStore::new(),
        }
    }
}

pub struct NexNode {
    pub identity: IdentityEngine,
    pub state: StateEngine,
    pub storage: StorageEngine,
    pub operational_state: NodeOperationalState,
    pub schema_version: u32,
}

impl NexNode {
    pub fn new(data_dir: impl Into<PathBuf>, signing_key: SigningKey) -> Self {
        let path = data_dir.into();
        let identity = IdentityEngine::new(signing_key);
        let node_id = hex::encode(&identity.actor_id[0..8]);
        let state = StateEngine::new(node_id);
        let storage = StorageEngine::new(path, None);

        Self {
            identity,
            state,
            storage,
            operational_state: NodeOperationalState::Uninitialized,
            schema_version: 1,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        fs::create_dir_all(&self.storage.data_dir)
            .map_err(|e| format!("Failed to create data dir: {:?}", e))?;

        // 1. Acquire PID lockfile with stale PID recovery
        let lock_path = self.storage.data_dir.join(".nex.lock");
        if lock_path.exists() {
            let mut pid_str = String::new();
            if let Ok(mut f) = File::open(&lock_path) {
                let _ = f.read_to_string(&mut pid_str);
            }
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    return Err(format!("Node lockfile exists: another daemon instance is active (PID {})", pid));
                }
            }
            // Stale or dead lockfile -> clean up
            let _ = fs::remove_file(&lock_path);
        }

        let my_pid = std::process::id();
        let mut lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|e| format!("Failed to create lockfile: {:?}", e))?;
        let _ = write!(lock_file, "{}", my_pid);
        let _ = lock_file.sync_all();

        // 2. Load Snapshot if present (Two-Phase Persistence)
        if let Ok(Some(snap)) = StateDbEngine::load_snapshot(&self.storage.data_dir) {
            self.state.current_epoch = snap.epoch;
            self.state.state_node.current_lamport = snap.lamport;
            self.state.latest_mutation_id = snap.latest_mutation_id;
            self.state.state_node.frontier = snap.frontier;
            self.state.state_node.crdt_state = snap.crdt_state;
            self.state.state_node.dag = snap.dag;
            self.state.object_store = snap.object_store;
            self.state.state_node.latest_checkpoint = snap.checkpoint;
            self.identity.blocklist.blocked_actors = snap.blocked_actors;
        }

        // 3. Replay WAL tail from disk and auto-truncate torn bytes
        self.operational_state = NodeOperationalState::ReplayingWal;
        let wal_path = self.storage.data_dir.join("wal.log");
        if let Ok(mutations) = WriteAheadLog::recover(&wal_path) {
            let mut affected_objects = std::collections::HashSet::new();
            for m in mutations {
                let target_obj_id = match &m.body.payload {
                    CrdtPayload::AddLWW { id, .. } | CrdtPayload::RemoveLWW { id } | CrdtPayload::Tombstone { id } => *id,
                };
                let (_disp, admitted_ids) = self.state.state_node.ingest_mutation_with_admissions(m.clone());
                self.state.latest_mutation_id = Some(m.id);
                if !admitted_ids.is_empty() {
                    affected_objects.insert(target_obj_id);
                }
            }

            // Derive object_store directly from crdt_state winners only for affected objects
            for obj_id in affected_objects {
                if let Some((_opt_val, epoch, lamport, winning_id)) = self.state.state_node.crdt_state.get(&obj_id) {
                    if let Some(winning_mutation) = self.state.state_node.dag.get(winning_id) {
                        match &winning_mutation.body.payload {
                            CrdtPayload::AddLWW { id, value } => {
                                if let Some(obj) = self.state.object_store.get_mut(id) {
                                    obj.payload_bytes = value.clone();
                                    obj.created_epoch = *epoch;
                                    obj.created_lamport = *lamport;
                                    obj.winning_mutation_id = *winning_id;
                                    obj.tombstoned = false;
                                } else {
                                    let obj = NexObject {
                                        object_id: *id,
                                        object_type: ObjectType::Synthetic(1),
                                        namespace: [0u8; 32],
                                        owner_actor_id: winning_mutation.body.author,
                                        schema_version: 1,
                                        created_epoch: *epoch,
                                        created_lamport: *lamport,
                                        winning_mutation_id: *winning_id,
                                        metadata: BTreeMap::new(),
                                        payload_bytes: value.clone(),
                                        tombstoned: false,
                                    };
                                    self.state.object_store.insert(*id, obj);
                                }
                            }
                            CrdtPayload::Tombstone { id } | CrdtPayload::RemoveLWW { id } => {
                                if let Some(obj) = self.state.object_store.get_mut(id) {
                                    obj.tombstoned = true;
                                    obj.created_epoch = *epoch;
                                    obj.created_lamport = *lamport;
                                    obj.winning_mutation_id = *winning_id;
                                } else {
                                    let obj = NexObject {
                                        object_id: *id,
                                        object_type: ObjectType::Synthetic(1),
                                        namespace: [0u8; 32],
                                        owner_actor_id: winning_mutation.body.author,
                                        schema_version: 1,
                                        created_epoch: *epoch,
                                        created_lamport: *lamport,
                                        winning_mutation_id: *winning_id,
                                        metadata: BTreeMap::new(),
                                        payload_bytes: Vec::new(),
                                        tombstoned: true,
                                    };
                                    self.state.object_store.insert(*id, obj);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Open fresh WAL file handle at the clean truncated end offset
        self.storage.wal = WriteAheadLog::open(&wal_path).ok();

        self.operational_state = NodeOperationalState::Running;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let lock_path = self.storage.data_dir.join(".nex.lock");
        if lock_path.exists() {
            let _ = fs::remove_file(&lock_path);
        }
        self.operational_state = NodeOperationalState::Stopped;
        Ok(())
    }

    pub fn checkpoint_and_compact(&mut self) -> Result<Checkpoint, CoreRuntimeError> {
        let checkpoint = self.state.state_node.compute_current_checkpoint();

        let snapshot = StateSnapshotData {
            epoch: self.state.current_epoch,
            lamport: self.state.state_node.current_lamport,
            latest_mutation_id: self.state.latest_mutation_id,
            frontier: self.state.state_node.frontier.clone(),
            crdt_state: self.state.state_node.crdt_state.clone(),
            dag: self.state.state_node.dag.clone(),
            object_store: self.state.object_store.clone(),
            checkpoint: Some(checkpoint.clone()),
            blocked_actors: self.identity.blocklist.blocked_actors.clone(),
        };

        // 1. Atomic Two-Phase Snapshot
        StateDbEngine::save_snapshot(&self.storage.data_dir, &snapshot)
            .map_err(|e| CoreRuntimeError::StorageError(format!("Failed to save state snapshot: {:?}", e)))?;

        // 2. Compact WAL
        self.storage.wal = None; // Close active file handle
        StateDbEngine::compact_wal(&self.storage.data_dir, &[])
            .map_err(|e| CoreRuntimeError::StorageError(format!("Failed to compact WAL: {:?}", e)))?;

        // 3. Reopen clean WAL
        let wal_path = self.storage.data_dir.join("wal.log");
        self.storage.wal = WriteAheadLog::open(wal_path).ok();

        Ok(checkpoint)
    }

    pub fn block_actor(&mut self, actor: ActorID) -> bool {
        self.identity.blocklist.block_actor(actor)
    }

    pub fn unblock_actor(&mut self, actor: &ActorID) -> bool {
        self.identity.blocklist.unblock_actor(actor)
    }

    pub fn is_actor_blocked(&self, actor: &ActorID) -> bool {
        self.identity.blocklist.is_blocked(actor)
    }

    pub fn execute_mutation(&mut self, body: MutationBody) -> Result<[u8; 32], CoreRuntimeError> {
        let m_id = hash_mutation_body(&body);
        let mutation = Mutation { id: m_id, body };

        // 1. Append to WAL (Disk first)
        if let Some(wal) = &mut self.storage.wal {
            wal.append_mutation(&mutation)
                .map_err(|e| CoreRuntimeError::StorageError(e.to_string()))?;
        }

        // 2. Ingest into DAG & CRDT evaluation
        self.state.state_node.ingest_mutation(mutation);
        self.state.latest_mutation_id = Some(m_id);

        Ok(m_id)
    }

    pub fn authorize_request(
        &self,
        target_namespace: &NamespaceID,
        target_object: Option<&ObjectID>,
        requested_op: u32,
        proof: Option<&CapabilityProof>,
    ) -> Result<ActorID, CoreRuntimeError> {
        if let Some(p) = proof {
            verify_capability_chain(
                p,
                requested_op,
                target_namespace,
                target_object,
                self.state.current_epoch,
                &self.identity.active_revocations,
                &self.identity.actor_id,
            ).map_err(|e| CoreRuntimeError::Unauthorized(format!("{:?}", e)))
        } else {
            Ok(self.identity.actor_id)
        }
    }

    pub fn gc_cas(&mut self, live_roots: &HashSet<[u8; 32]>) -> usize {
        let mut to_remove = Vec::new();
        for (digest, _) in &self.storage.cas.chunks {
            if !live_roots.contains(digest) {
                to_remove.push(*digest);
            }
        }
        let count = to_remove.len();
        for digest in to_remove {
            self.storage.cas.chunks.remove(&digest);
        }
        count
    }
}

impl NexAppApi for NexNode {
    fn create_object(
        &mut self,
        namespace: NamespaceID,
        object_type: ObjectType,
        metadata: BTreeMap<String, String>,
        payload: Vec<u8>,
    ) -> Result<ObjectID, CoreRuntimeError> {
        if payload.len() > crate::api::MAX_OBJECT_PAYLOAD_BYTES {
            return Err(CoreRuntimeError::InvalidPayload(format!("Payload size {} exceeds limit of {} bytes", payload.len(), crate::api::MAX_OBJECT_PAYLOAD_BYTES)));
        }
        let meta_len: usize = metadata.iter().map(|(k, v)| k.len() + v.len()).sum();
        if meta_len > crate::api::MAX_OBJECT_METADATA_BYTES {
            return Err(CoreRuntimeError::InvalidPayload(format!("Metadata size {} exceeds limit of {} bytes", meta_len, crate::api::MAX_OBJECT_METADATA_BYTES)));
        }

        self.authorize_request(&namespace, None, OP_REGISTER_LWW, None)?;

        let parents = self.state.latest_mutation_id.map(|id| vec![id]).unwrap_or_default();
        let lamport = if parents.is_empty() { 0 } else { self.state.state_node.current_lamport + 1 };

        let mut hasher = sha2::Sha256::default();
        use sha2::Digest;
        hasher.update(b"NEX/OBJECT_ID/v1");
        hasher.update(&namespace);
        hasher.update(&self.identity.actor_id);
        hasher.update(&lamport.to_le_bytes());
        hasher.update(&payload);
        let object_id: [u8; 32] = hasher.finalize().into();

        let body = MutationBody {
            author: self.identity.actor_id,
            parents,
            lamport,
            epoch: self.state.current_epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: object_id, value: payload.clone() },
        };

        let m_id = self.execute_mutation(body)?;

        let obj = NexObject {
            object_id,
            object_type,
            namespace,
            owner_actor_id: self.identity.actor_id,
            schema_version: 1,
            created_epoch: self.state.current_epoch,
            created_lamport: lamport,
            winning_mutation_id: m_id,
            metadata,
            payload_bytes: payload,
            tombstoned: false,
        };
        self.state.object_store.insert(object_id, obj);

        Ok(object_id)
    }

    fn mutate_object(
        &mut self,
        object_id: ObjectID,
        new_metadata: Option<BTreeMap<String, String>>,
        new_payload: Option<Vec<u8>>,
        proof: Option<CapabilityProof>,
    ) -> Result<[u8; 32], CoreRuntimeError> {
        if let Some(p) = &new_payload {
            if p.len() > crate::api::MAX_OBJECT_PAYLOAD_BYTES {
                return Err(CoreRuntimeError::InvalidPayload(format!("Payload size {} exceeds limit of {} bytes", p.len(), crate::api::MAX_OBJECT_PAYLOAD_BYTES)));
            }
        }
        if let Some(m) = &new_metadata {
            let meta_len: usize = m.iter().map(|(k, v)| k.len() + v.len()).sum();
            if meta_len > crate::api::MAX_OBJECT_METADATA_BYTES {
                return Err(CoreRuntimeError::InvalidPayload(format!("Metadata size {} exceeds limit of {} bytes", meta_len, crate::api::MAX_OBJECT_METADATA_BYTES)));
            }
        }

        let existing = self.state.object_store.get(&object_id)
            .ok_or(CoreRuntimeError::ObjectNotFound(object_id))?;
        if existing.tombstoned {
            return Err(CoreRuntimeError::ObjectTombstoned);
        }
        let namespace = existing.namespace;

        self.authorize_request(&namespace, Some(&object_id), OP_WRITE, proof.as_ref())?;

        let payload_bytes = new_payload.unwrap_or_else(|| existing.payload_bytes.clone());
        let parents = self.state.latest_mutation_id.map(|id| vec![id]).unwrap_or_default();
        let lamport = if parents.is_empty() { 0 } else { self.state.state_node.current_lamport + 1 };

        let body = MutationBody {
            author: self.identity.actor_id,
            parents,
            lamport,
            epoch: self.state.current_epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: object_id, value: payload_bytes.clone() },
        };

        let m_id = self.execute_mutation(body)?;

        if let Some(obj) = self.state.object_store.get_mut(&object_id) {
            if let Some(m) = new_metadata {
                obj.metadata = m;
            }
            obj.payload_bytes = payload_bytes;
            obj.created_epoch = self.state.current_epoch;
            obj.created_lamport = lamport;
            obj.winning_mutation_id = m_id;
        }

        Ok(m_id)
    }

    fn read_object(&self, object_id: &ObjectID) -> Result<NexObject, CoreRuntimeError> {
        let obj = self.state.object_store.get(object_id)
            .ok_or(CoreRuntimeError::ObjectNotFound(*object_id))?;
        if obj.tombstoned {
            return Err(CoreRuntimeError::ObjectTombstoned);
        }
        Ok(obj.clone())
    }

    fn delete_object(
        &mut self,
        object_id: ObjectID,
        proof: Option<CapabilityProof>,
    ) -> Result<[u8; 32], CoreRuntimeError> {
        let existing = self.state.object_store.get(&object_id)
            .ok_or(CoreRuntimeError::ObjectNotFound(object_id))?;
        let namespace = existing.namespace;

        self.authorize_request(&namespace, Some(&object_id), OP_OBJECT_TOMBSTONE, proof.as_ref())?;

        let parents = self.state.latest_mutation_id.map(|id| vec![id]).unwrap_or_default();
        let lamport = if parents.is_empty() { 0 } else { self.state.state_node.current_lamport + 1 };

        let body = MutationBody {
            author: self.identity.actor_id,
            parents,
            lamport,
            epoch: self.state.current_epoch,
            is_resurrect: false,
            payload: CrdtPayload::Tombstone { id: object_id },
        };

        let m_id = self.execute_mutation(body)?;
        if let Some(obj) = self.state.object_store.get_mut(&object_id) {
            obj.tombstoned = true;
            obj.created_epoch = self.state.current_epoch;
            obj.created_lamport = lamport;
            obj.winning_mutation_id = m_id;
        }

        Ok(m_id)
    }

    fn delegate_capability(
        &mut self,
        subject: ActorID,
        namespace: NamespaceID,
        object_id: Option<ObjectID>,
        allowed_ops: u32,
        valid_epochs: (u64, u64),
    ) -> Result<CapabilityProof, CoreRuntimeError> {
        let token = CapabilityToken {
            issuer: self.identity.actor_id,
            subject,
            namespace,
            object_id,
            allowed_operations: allowed_ops,
            delegation_depth: 2,
            not_before_epoch: valid_epochs.0,
            expires_at_epoch: valid_epochs.1,
            parent_token_hash: None,
        };

        let token_hash = hash_capability_token(&token);
        let sig = self.identity.signing_key.sign(&token_hash).to_bytes().to_vec();

        Ok(CapabilityProof {
            token,
            parent_proof: None,
            issuer_pubkey: Some(self.identity.pubkey_bytes.clone()),
            signature: sig,
        })
    }

    fn sync_now(&mut self) -> Result<Checkpoint, CoreRuntimeError> {
        Ok(self.state.state_node.compute_current_checkpoint())
    }
}

pub struct SovereignNodeRuntime {
    pub actor_id: ActorID,
    pub state_node: VirtualNode,
    pub routing_table: RoutingTable,
    pub transport: MultiTransportDispatcher,
    pub rate_limiter: PeerRateLimiter,
    pub peer_jail: PeerJail,
    pub preflight_shield: PreFlightShield,
    pub reassembler: FragmentationReassembler,
    pub drive: DriveEngine,
    pub chat: ChatEngine,
    pub community: CommunityEngine,
    pub current_epoch: u64,
}

impl SovereignNodeRuntime {
    pub fn new(
        actor_id: ActorID,
        namespace_id: [u8; 32],
        expected_image_id: [u8; 32],
    ) -> Self {
        Self {
            actor_id,
            state_node: VirtualNode::new(hex::encode(&actor_id[0..8])),
            routing_table: RoutingTable::new(actor_id),
            transport: MultiTransportDispatcher::new(),
            rate_limiter: PeerRateLimiter::new(100, 10),
            peer_jail: PeerJail::new(),
            preflight_shield: PreFlightShield::new(expected_image_id),
            reassembler: FragmentationReassembler::new(),
            drive: DriveEngine::new(namespace_id),
            chat: ChatEngine::new(),
            community: CommunityEngine::new(namespace_id, actor_id),
            current_epoch: 0,
        }
    }

    pub fn submit_local_mutation(&mut self, m: Mutation) {
        self.state_node.ingest_mutation(m);
    }

    pub fn tick(&mut self, current_epoch: u64) -> Vec<crate::sync::types::IngressDisposition> {
        self.current_epoch = current_epoch;
        let mut dispositions = Vec::new();
        let packets = self.transport.poll_all_incoming();

        for packet in packets {
            let mut sender_actor = [0u8; 32];
            if packet.source_address.len() >= 32 {
                sender_actor.copy_from_slice(&packet.source_address[0..32]);
            }

            match crate::transport::types::decode_frame(&packet.payload) {
                Ok((_tag, _flags, inner_bytes)) => {
                    if let Ok(Some(complete_payload)) = self.reassembler.ingest_chunk_with_epoch(&inner_bytes, current_epoch) {
                        if let Ok((_inner_tag, _inner_flags, frame_data)) = crate::transport::types::decode_frame(&complete_payload) {
                            if let Ok(sync_msg) = serde_json::from_slice::<crate::sync::types::SyncMessage>(&frame_data) {
                                match sync_msg {
                                    crate::sync::types::SyncMessage::DirectMutationBroadcast(m) => {
                                        let disp = self.state_node.ingest_mutation(m);
                                        dispositions.push(disp);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    self.peer_jail.record_penalty(&sender_actor, 10, current_epoch);
                }
            }
        }

        dispositions
    }

    pub fn checkpoint(&mut self) -> Checkpoint {
        self.state_node.compute_current_checkpoint()
    }
}

