use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use ed25519_dalek::{SigningKey, Signer};
use crate::identity::types::{
    ActorID, KeyType, CapabilityToken, CapabilityProof, AuthorizationError,
    OP_REGISTER_LWW, OP_OBJECT_TOMBSTONE, OP_ALL
};
use crate::identity::verifier::{derive_actor_id, hash_capability_token, verify_capability_chain};
use crate::object::types::{ObjectID, NamespaceID, ObjectType, NexObject};
use crate::object::store::NexObjectStore;
use crate::sync::node::VirtualNode;
use crate::storage::wal::WriteAheadLog;
use crate::model::{Mutation, MutationBody, CrdtPayload, Checkpoint};
use crate::hash::hash_mutation_body;

pub const MAX_OBJECT_PAYLOAD_BYTES: usize = 2 * 1024 * 1024; // 2 MB
pub const MAX_OBJECT_METADATA_BYTES: usize = 64 * 1024; // 64 KB

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreRuntimeError {
    Unauthorized(String),
    ObjectNotFound(ObjectID),
    InvalidSchema(String),
    StorageError(String),
    ObjectTombstoned,
    InvalidPayload(String),
    ResourceExhausted(String),
}

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

impl<'a, T: NexAppApi + ?Sized> NexAppApi for &'a mut T {
    fn create_object(
        &mut self,
        namespace: NamespaceID,
        object_type: ObjectType,
        metadata: BTreeMap<String, String>,
        payload: Vec<u8>,
    ) -> Result<ObjectID, CoreRuntimeError> {
        (**self).create_object(namespace, object_type, metadata, payload)
    }

    fn mutate_object(
        &mut self,
        object_id: ObjectID,
        new_metadata: Option<BTreeMap<String, String>>,
        new_payload: Option<Vec<u8>>,
        proof: Option<CapabilityProof>,
    ) -> Result<[u8; 32], CoreRuntimeError> {
        (**self).mutate_object(object_id, new_metadata, new_payload, proof)
    }

    fn read_object(&self, object_id: &ObjectID) -> Result<NexObject, CoreRuntimeError> {
        (**self).read_object(object_id)
    }

    fn delete_object(
        &mut self,
        object_id: ObjectID,
        proof: Option<CapabilityProof>,
    ) -> Result<[u8; 32], CoreRuntimeError> {
        (**self).delete_object(object_id, proof)
    }

    fn delegate_capability(
        &mut self,
        subject: ActorID,
        namespace: NamespaceID,
        object_id: Option<ObjectID>,
        allowed_ops: u32,
        valid_epochs: (u64, u64),
    ) -> Result<CapabilityProof, CoreRuntimeError> {
        (**self).delegate_capability(subject, namespace, object_id, allowed_ops, valid_epochs)
    }

    fn sync_now(&mut self) -> Result<Checkpoint, CoreRuntimeError> {
        (**self).sync_now()
    }
}

pub struct NexCoreRuntime {
    pub actor_id: ActorID,
    pub signing_key: SigningKey,
    pub pubkey_bytes: Vec<u8>,
    pub object_store: NexObjectStore,
    pub state_node: VirtualNode,
    pub active_revocations: BTreeMap<[u8; 32], u64>,
    pub wal: Option<WriteAheadLog>,
    pub current_epoch: u64,
    pub latest_mutation_id: Option<[u8; 32]>,
}

impl NexCoreRuntime {
    pub fn new(signing_key: SigningKey, wal_path: Option<PathBuf>) -> Self {
        let pubkey_bytes = signing_key.verifying_key().to_bytes().to_vec();
        let actor_id = derive_actor_id(KeyType::Ed25519, &pubkey_bytes);
        let wal = wal_path.and_then(|p| WriteAheadLog::open(p).ok());

        Self {
            actor_id,
            signing_key,
            pubkey_bytes,
            object_store: NexObjectStore::new(),
            state_node: VirtualNode::new("NexCoreRuntimeNode"),
            active_revocations: BTreeMap::new(),
            wal,
            current_epoch: 0,
            latest_mutation_id: None,
        }
    }

    fn authorize_request(
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
                self.current_epoch,
                &self.active_revocations,
                &self.actor_id,
            ).map_err(|e| CoreRuntimeError::Unauthorized(format!("{:?}", e)))
        } else {
            // Master node authority
            Ok(self.actor_id)
        }
    }
}

impl NexAppApi for NexCoreRuntime {
    fn create_object(
        &mut self,
        namespace: NamespaceID,
        object_type: ObjectType,
        metadata: BTreeMap<String, String>,
        payload: Vec<u8>,
    ) -> Result<ObjectID, CoreRuntimeError> {
        // 1. Authorize creation
        self.authorize_request(&namespace, None, OP_REGISTER_LWW, None)?;

        // 2. Generate deterministic ObjectID
        let mut hasher = sha2::Sha256::default();
        use sha2::Digest;
        hasher.update(b"NEX/OBJECT_ID/v1");
        hasher.update(&namespace);
        hasher.update(&self.actor_id);
        hasher.update(&(self.state_node.current_lamport + 1).to_le_bytes());
        hasher.update(&payload);
        let object_id: [u8; 32] = hasher.finalize().into();

        // 3. Build Mutation
        let parents = self.latest_mutation_id.map(|id| vec![id]).unwrap_or_default();
        let body = MutationBody {
            author: self.actor_id,
            parents,
            lamport: self.state_node.current_lamport + 1,
            epoch: self.current_epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: object_id, value: payload.clone() },
        };
        let m_id = hash_mutation_body(&body);
        let mutation = Mutation { id: m_id, body };

        // 4. Commit to WAL & in-memory state
        if let Some(wal) = &mut self.wal {
            wal.append_mutation(&mutation)
                .map_err(|e| CoreRuntimeError::StorageError(e.to_string()))?;
        }
        self.state_node.ingest_mutation(mutation);
        self.latest_mutation_id = Some(m_id);

        // 5. Store NexObject in ObjectStore
        let obj = NexObject {
            object_id,
            object_type,
            namespace,
            owner_actor_id: self.actor_id,
            schema_version: 1,
            created_epoch: self.current_epoch,
            created_lamport: self.state_node.current_lamport,
            winning_mutation_id: m_id,
            metadata,
            payload_bytes: payload,
            tombstoned: false,
        };
        self.object_store.insert(obj);

        Ok(object_id)
    }

    fn mutate_object(
        &mut self,
        object_id: ObjectID,
        new_metadata: Option<BTreeMap<String, String>>,
        new_payload: Option<Vec<u8>>,
        proof: Option<CapabilityProof>,
    ) -> Result<[u8; 32], CoreRuntimeError> {
        let existing = self.object_store.get(&object_id)
            .ok_or(CoreRuntimeError::ObjectNotFound(object_id))?;
        if existing.tombstoned {
            return Err(CoreRuntimeError::ObjectTombstoned);
        }
        let namespace = existing.namespace;

        // 1. Authorize mutation
        self.authorize_request(&namespace, Some(&object_id), OP_REGISTER_LWW, proof.as_ref())?;

        // 2. Build Mutation
        let payload_val = new_payload.unwrap_or_else(|| existing.payload_bytes.clone());
        let parents = self.latest_mutation_id.map(|id| vec![id]).unwrap_or_default();
        let lamport = self.state_node.current_lamport + 1;
        let body = MutationBody {
            author: self.actor_id,
            parents,
            lamport,
            epoch: self.current_epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: object_id, value: payload_val.clone() },
        };
        let m_id = hash_mutation_body(&body);
        let mutation = Mutation { id: m_id, body };

        // 3. Commit to WAL & DAG
        if let Some(wal) = &mut self.wal {
            wal.append_mutation(&mutation)
                .map_err(|e| CoreRuntimeError::StorageError(e.to_string()))?;
        }
        self.state_node.ingest_mutation(mutation);
        self.latest_mutation_id = Some(m_id);

        // 4. Update in ObjectStore
        if let Some(obj) = self.object_store.get_mut(&object_id) {
            if let Some(meta) = new_metadata {
                obj.metadata = meta;
            }
            obj.payload_bytes = payload_val;
            obj.created_epoch = self.current_epoch;
            obj.created_lamport = lamport;
            obj.winning_mutation_id = m_id;
        }

        Ok(m_id)
    }

    fn read_object(&self, object_id: &ObjectID) -> Result<NexObject, CoreRuntimeError> {
        let obj = self.object_store.get(object_id)
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
        let existing = self.object_store.get(&object_id)
            .ok_or(CoreRuntimeError::ObjectNotFound(object_id))?;
        let namespace = existing.namespace;

        // 1. Authorize deletion
        self.authorize_request(&namespace, Some(&object_id), OP_OBJECT_TOMBSTONE, proof.as_ref())?;

        // 2. Build Tombstone Mutation
        let parents = self.latest_mutation_id.map(|id| vec![id]).unwrap_or_default();
        let body = MutationBody {
            author: self.identity.actor_id,
            parents,
            lamport: self.state_node.current_lamport + 1,
            epoch: self.current_epoch,
            is_resurrect: false,
            payload: CrdtPayload::Tombstone { id: object_id },
        };
        let m_id = hash_mutation_body(&body);
        let mutation = Mutation { id: m_id, body };

        // 3. Commit to WAL & DAG
        if let Some(wal) = &mut self.wal {
            wal.append_mutation(&mutation)
                .map_err(|e| CoreRuntimeError::StorageError(e.to_string()))?;
        }
        self.state_node.ingest_mutation(mutation);
        self.latest_mutation_id = Some(m_id);

        // 4. Tombstone in ObjectStore
        self.object_store.tombstone(&object_id);

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
            issuer: self.actor_id,
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
        let signature = self.signing_key.sign(&token_hash).to_bytes().to_vec();

        Ok(CapabilityProof {
            token,
            parent_proof: None,
            issuer_pubkey: Some(self.pubkey_bytes.clone()),
            signature,
        })
    }

    fn sync_now(&mut self) -> Result<Checkpoint, CoreRuntimeError> {
        Ok(self.state_node.compute_current_checkpoint())
    }
}
