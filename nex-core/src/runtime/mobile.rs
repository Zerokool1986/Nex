use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::{BTreeMap, BTreeSet};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Signature};
use crate::runtime::node::NexNode;
use crate::runtime::production::NodeOperationalState;
use crate::api::NexAppApi;
use crate::identity::types::{CapabilityProof, ActorID, NamespaceID, ObjectID, OP_READ, DeviceCertificate};
use crate::identity::verifier::{verify_capability_chain, verify_device_certificate_with_crl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicePowerState {
    Active,
    DozeStandby,
    BatterySaverThrottled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidLifecycleState {
    Uninitialized,
    Foreground,
    Background,
    Doze,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidScopedConfig {
    pub package_name: String,
    pub internal_files_dir: PathBuf,
    pub external_cache_dir: Option<PathBuf>,
}

pub struct AndroidPlatformAdapter {
    pub config: AndroidScopedConfig,
    pub power_state: DevicePowerState,
    pub is_network_metered: bool,
    pub is_sync_in_progress: Arc<AtomicBool>,
}

impl AndroidPlatformAdapter {
    pub fn new(package_name: &str, files_dir: PathBuf) -> Self {
        Self {
            config: AndroidScopedConfig {
                package_name: package_name.to_string(),
                internal_files_dir: files_dir,
                external_cache_dir: None,
            },
            power_state: DevicePowerState::Active,
            is_network_metered: false,
            is_sync_in_progress: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn on_doze_entered(&mut self) {
        self.power_state = DevicePowerState::DozeStandby;
    }

    pub fn on_doze_exited(&mut self) {
        self.power_state = DevicePowerState::Active;
    }

    pub fn set_battery_saver(&mut self, enabled: bool) {
        if enabled {
            self.power_state = DevicePowerState::BatterySaverThrottled;
        } else {
            self.power_state = DevicePowerState::Active;
        }
    }

    pub fn calculate_max_batch_size(&self) -> usize {
        match self.power_state {
            DevicePowerState::Active => 100,
            DevicePowerState::DozeStandby => 10,
            DevicePowerState::BatterySaverThrottled => 25,
        }
    }

    pub fn trigger_workmanager_sync(&self, node: &mut NexNode) -> Result<[u8; 32], String> {
        if self.power_state == DevicePowerState::DozeStandby {
            return Err("SyncDeferred: device is in deep Doze standby mode".into());
        }

        self.is_sync_in_progress.store(true, Ordering::SeqCst);
        let cp = node.sync_now().map_err(|e| format!("{:?}", e))?;
        self.is_sync_in_progress.store(false, Ordering::SeqCst);
        Ok(cp.body.state_root)
    }
}

pub struct DesktopPlatformAdapter {
    pub app_data_dir: PathBuf,
    pub service_name: String,
}

impl DesktopPlatformAdapter {
    pub fn new(service_name: &str, app_data_dir: PathBuf) -> Self {
        Self {
            app_data_dir,
            service_name: service_name.to_string(),
        }
    }

    pub fn poll_daemon_health(&self, node: &NexNode) -> Result<String, String> {
        if node.operational_state == NodeOperationalState::Running {
            Ok(format!("Running [PID: {}]", std::process::id()))
        } else {
            Err("Daemon is not active".into())
        }
    }
}

// -----------------------------------------------------------------------------
// GATE R71-2: ANDROID HOST FOUNDATION MODULES
// -----------------------------------------------------------------------------

/// Coordinates Android host application & activity lifecycles.
pub struct AndroidHostCoordinator {
    pub package_name: String,
    pub data_dir: PathBuf,
    pub state: AndroidLifecycleState,
    pub low_memory_warning_count: u32,
}

impl AndroidHostCoordinator {
    pub fn new(package_name: &str, data_dir: PathBuf) -> Self {
        Self {
            package_name: package_name.to_string(),
            data_dir,
            state: AndroidLifecycleState::Uninitialized,
            low_memory_warning_count: 0,
        }
    }

    pub fn on_app_create(&mut self) -> Result<(), String> {
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        self.state = AndroidLifecycleState::Foreground;
        Ok(())
    }

    pub fn on_app_pause(&mut self) {
        self.state = AndroidLifecycleState::Background;
    }

    pub fn on_app_resume(&mut self) {
        self.state = AndroidLifecycleState::Foreground;
    }

    pub fn on_low_memory(&mut self, node: &mut NexNode) -> Result<(), String> {
        self.low_memory_warning_count += 1;
        // On Low Memory warning: flush memory caches and fsync WAL
        node.sync_now().map_err(|e| format!("{:?}", e))?;
        Ok(())
    }

    pub fn on_app_terminate(&mut self, node: &mut NexNode) -> Result<(), String> {
        self.state = AndroidLifecycleState::Terminated;
        node.stop().map_err(|e| format!("{:?}", e))?;
        Ok(())
    }
}

/// Simulated / TEE-backed Android KeyStore Device Signing Broker.
pub struct AndroidKeystoreBroker {
    device_signing_key: SigningKey,
    pub device_verifying_key: VerifyingKey,
    pub certificate: Option<DeviceCertificate>,
}

impl AndroidKeystoreBroker {
    pub fn generate_in_keystore(device_seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(device_seed);
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
        let expected_device_actor = crate::identity::verifier::derive_actor_id(
            crate::identity::types::KeyType::Ed25519,
            &self.device_verifying_key.to_bytes(),
        );
        if cert.device_actor_id != expected_device_actor {
            return Err("Keystore device key does not match certificate".into());
        }
        verify_device_certificate_with_crl(cert, root_actor, current_epoch, crl)
            .map_err(|e| format!("{:?}", e))
    }
}

/// Power-aware WorkManager background synchronization manager.
pub struct AndroidWorkManagerScheduler {
    pub requires_charging: bool,
    pub requires_unmetered: bool,
    pub pending_sync_items: Vec<ObjectID>,
}

impl AndroidWorkManagerScheduler {
    pub fn new(requires_charging: bool, requires_unmetered: bool) -> Self {
        Self {
            requires_charging,
            requires_unmetered,
            pending_sync_items: Vec::new(),
        }
    }

    pub fn enqueue_sync_target(&mut self, object_id: ObjectID) {
        if !self.pending_sync_items.contains(&object_id) {
            self.pending_sync_items.push(object_id);
        }
    }

    pub fn can_execute(&self, is_charging: bool, is_unmetered: bool, power_state: DevicePowerState) -> bool {
        if power_state == DevicePowerState::DozeStandby {
            return false;
        }
        if self.requires_charging && !is_charging {
            return false;
        }
        if self.requires_unmetered && !is_unmetered {
            return false;
        }
        true
    }

    pub fn execute_scheduled_sync(&mut self, node: &mut NexNode) -> Result<usize, String> {
        let count = self.pending_sync_items.len();
        node.sync_now().map_err(|e| format!("{:?}", e))?;
        self.pending_sync_items.clear();
        Ok(count)
    }
}

/// Zero-Ambient Capability Gateway for Android Components.
pub struct AndroidCapabilityGateway;

impl AndroidCapabilityGateway {
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
}
