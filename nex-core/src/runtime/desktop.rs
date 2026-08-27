use std::path::PathBuf;
use std::collections::{BTreeMap, BTreeSet};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Signature};
use sha2::{Sha256, Digest};
use crate::runtime::node::NexNode;
use crate::runtime::production::NodeOperationalState;
use crate::identity::types::{CapabilityProof, ActorID, NamespaceID, ObjectID, OP_READ, OP_WRITE, DeviceCertificate, KeyType};
use crate::identity::verifier::{verify_capability_chain, verify_device_certificate_with_crl, derive_actor_id};
use crate::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};
use crate::object::types::{ObjectType, NexObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopLifecycleState {
    Uninitialized,
    Starting,
    Running,
    BackgroundTray,
    Stopping,
    Terminated,
}

/// Desktop Host Lifecycle & Multi-Window Coordinator.
pub struct DesktopHostCoordinator {
    pub app_name: String,
    pub data_dir: PathBuf,
    pub state: DesktopLifecycleState,
    pub active_windows: u32,
}

impl DesktopHostCoordinator {
    pub fn new(app_name: &str, data_dir: PathBuf) -> Self {
        Self {
            app_name: app_name.to_string(),
            data_dir,
            state: DesktopLifecycleState::Uninitialized,
            active_windows: 0,
        }
    }

    pub fn on_app_start(&mut self, node: &mut NexNode) -> Result<(), String> {
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        self.state = DesktopLifecycleState::Starting;
        node.start().map_err(|e| format!("{:?}", e))?;
        self.state = DesktopLifecycleState::Running;
        self.active_windows = 1; // Primary window
        Ok(())
    }

    pub fn on_window_opened(&mut self) {
        self.active_windows += 1;
        if self.state == DesktopLifecycleState::BackgroundTray {
            self.state = DesktopLifecycleState::Running;
        }
    }

    pub fn on_window_closed(&mut self) -> DesktopLifecycleState {
        if self.active_windows > 0 {
            self.active_windows -= 1;
        }
        if self.active_windows == 0 {
            self.state = DesktopLifecycleState::BackgroundTray;
        }
        self.state
    }

    pub fn on_app_stop(&mut self, node: &mut NexNode) -> Result<(), String> {
        self.state = DesktopLifecycleState::Stopping;
        node.stop().map_err(|e| format!("{:?}", e))?;
        self.state = DesktopLifecycleState::Terminated;
        self.active_windows = 0;
        Ok(())
    }
}

/// Authenticated Local RPC Broker for auxiliary windows and CLI clients.
pub struct DesktopLocalRpcBroker {
    auth_token: [u8; 32],
}

impl DesktopLocalRpcBroker {
    pub fn new(auth_token: [u8; 32]) -> Self {
        Self { auth_token }
    }

    pub fn authenticate(&self, bearer_token: &[u8; 32]) -> bool {
        self.auth_token == *bearer_token
    }

    pub fn dispatch_authenticated(
        &self,
        node: &mut NexNode,
        bearer_token: &[u8; 32],
        req_json_bytes: &[u8],
    ) -> Result<Vec<u8>, String> {
        if !self.authenticate(bearer_token) {
            return Err("UnauthorizedRpc: Invalid bearer authentication token".into());
        }

        let req_str = std::str::from_utf8(req_json_bytes).map_err(|e| format!("InvalidUtf8: {:?}", e))?;
        let req_obj: JsonRpcRequest = serde_json::from_str(req_str).map_err(|e| format!("InvalidJsonRpc: {:?}", e))?;

        let resp_obj = NexRpcDispatcher::dispatch_node(node, req_obj);
        let resp_str = serde_json::to_string(&resp_obj).map_err(|e| format!("SerializationError: {:?}", e))?;
        Ok(resp_str.into_bytes())
    }
}

/// OS Keyring / Credential Guard / Keychain Integration Broker.
pub struct DesktopKeyringBroker {
    device_signing_key: SigningKey,
    pub device_verifying_key: VerifyingKey,
    pub certificate: Option<DeviceCertificate>,
}

impl DesktopKeyringBroker {
    pub fn init_from_secret_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            device_signing_key: signing_key,
            device_verifying_key: verifying_key,
            certificate: None,
        }
    }

    pub fn set_certificate(&mut self, cert: DeviceCertificate) {
        self.certificate = Some(cert);
    }

    pub fn sign_payload(&self, payload: &[u8]) -> Result<[u8; 64], String> {
        let sig: Signature = self.device_signing_key.sign(payload);
        Ok(sig.to_bytes())
    }

    pub fn verify_device_authorization(&self, root_actor: &ActorID, current_epoch: u64, crl: &BTreeSet<ActorID>) -> Result<(), String> {
        let cert = self.certificate.as_ref().ok_or("No DeviceCertificate provisioned")?;
        let expected_device_actor = derive_actor_id(KeyType::Ed25519, &self.device_verifying_key.to_bytes());
        if cert.device_actor_id != expected_device_actor {
            return Err("Keyring device key does not match certificate".into());
        }
        verify_device_certificate_with_crl(cert, root_actor, current_epoch, crl)
            .map_err(|e| format!("{:?}", e))
    }
}

/// Desktop Zero-Ambient Capability Gateway.
pub struct DesktopCapabilityGateway;

impl DesktopCapabilityGateway {
    pub fn authorize_and_read(
        node: &NexNode,
        actor_id: &ActorID,
        namespace_id: &NamespaceID,
        object_id: &ObjectID,
        proof: &CapabilityProof,
        current_epoch: u64,
        revoked_delegations: &BTreeMap<ActorID, u64>,
    ) -> Result<Vec<u8>, String> {
        verify_capability_chain(
            proof,
            OP_READ,
            namespace_id,
            Some(object_id),
            current_epoch,
            revoked_delegations,
            actor_id,
        ).map_err(|e| format!("CapabilityDenied: {:?}", e))?;

        match node.state.object_store.get(object_id) {
            Some(obj) => Ok(obj.payload_bytes.clone()),
            None => Err("ObjectNotFound".into()),
        }
    }

    pub fn authorize_and_write(
        node: &mut NexNode,
        actor_id: &ActorID,
        namespace_id: &NamespaceID,
        object_id: &ObjectID,
        object_type: ObjectType,
        payload_bytes: Vec<u8>,
        proof: &CapabilityProof,
        current_epoch: u64,
        revoked_delegations: &BTreeMap<ActorID, u64>,
    ) -> Result<(), String> {
        verify_capability_chain(
            proof,
            OP_WRITE,
            namespace_id,
            Some(object_id),
            current_epoch,
            revoked_delegations,
            actor_id,
        ).map_err(|e| format!("CapabilityDenied: {:?}", e))?;

        let obj = NexObject {
            object_id: *object_id,
            object_type,
            namespace: *namespace_id,
            owner_actor_id: *actor_id,
            schema_version: 1,
            created_epoch: current_epoch,
            created_lamport: current_epoch,
            winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes,
            tombstoned: false,
        };

        node.state.object_store.insert(*object_id, obj);
        Ok(())
    }
}
