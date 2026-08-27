use std::collections::{BTreeMap, BTreeSet};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use crate::object::types::{ObjectID, NamespaceID, ObjectType, NexObject};
use crate::api::{NexAppApi, CoreRuntimeError};
use crate::identity::types::{ActorID, CapabilityProof, KeyType, OP_READ, OP_WRITE, OP_ALL};
use crate::identity::verifier::derive_actor_id;

pub const DOMAIN_MANIFEST: &[u8] = b"NEX/MANIFEST/v1";
pub const DOMAIN_APP_NS:   &[u8] = b"NEX/APP_NS/v1";
pub const DOMAIN_SCHEMA:   &[u8] = b"NEX/SCHEMA/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomObjectTypeRegistration {
    pub type_id: u16, // 0x1000..0xEFFF
    pub name: String,
    pub schema_definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppCapabilityRequest {
    pub namespace_scope: String,
    pub allowed_operations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppManifest {
    pub manifest_version: String,
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub min_nex_core_version: String,
    pub developer_actor_id: ActorID,
    pub developer_signature: Vec<u8>,
    pub requested_capabilities: Vec<AppCapabilityRequest>,
    pub registered_object_types: Vec<CustomObjectTypeRegistration>,
}

impl AppManifest {
    pub fn compute_canonical_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_MANIFEST);
        hasher.update(self.app_id.as_bytes());
        hasher.update(self.version.as_bytes());
        hasher.update(&self.developer_actor_id);
        for reg in &self.registered_object_types {
            hasher.update(&reg.type_id.to_le_bytes());
            hasher.update(reg.name.as_bytes());
        }
        hasher.finalize().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppResourceQuota {
    pub max_memory_bytes: usize,
    pub max_fuel: u64,
    pub max_storage_bytes: u64,
    pub current_storage_bytes: u64,
}

impl Default for AppResourceQuota {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB
            max_fuel: 50_000_000,                // 50M instructions
            max_storage_bytes: 5 * 1024 * 1024 * 1024, // 5 GB
            current_storage_bytes: 0,
        }
    }
}

pub struct NexExtensionHost<A: NexAppApi> {
    pub user_actor_id: ActorID,
    pub api: A,
    pub installed_apps: BTreeMap<String, AppManifest>,
    pub app_namespaces: BTreeMap<String, NamespaceID>,
    pub registered_schemas: BTreeMap<u16, (String, [u8; 32])>, // TypeID -> (AppID, SchemaDigest)
    pub quotas: BTreeMap<String, AppResourceQuota>,
}

impl<A: NexAppApi> NexExtensionHost<A> {
    pub fn new(user_actor_id: ActorID, api: A) -> Self {
        Self {
            user_actor_id,
            api,
            installed_apps: BTreeMap::new(),
            app_namespaces: BTreeMap::new(),
            registered_schemas: BTreeMap::new(),
            quotas: BTreeMap::new(),
        }
    }

    pub fn derive_app_namespace(&self, app_id: &str) -> NamespaceID {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_APP_NS);
        hasher.update(&self.user_actor_id);
        hasher.update(app_id.as_bytes());
        hasher.finalize().into()
    }

    pub fn install_app(
        &mut self,
        manifest: AppManifest,
        developer_pubkey_bytes: &[u8; 32],
    ) -> Result<NamespaceID, String> {
        // 1. Verify Developer Signature
        let expected_actor = derive_actor_id(KeyType::Ed25519, developer_pubkey_bytes);
        if manifest.developer_actor_id != expected_actor {
            return Err("Developer ActorID does not match provided public key".into());
        }

        let verifier_key = VerifyingKey::from_bytes(developer_pubkey_bytes)
            .map_err(|e| format!("Invalid developer pubkey: {:?}", e))?;
        let sig_bytes: [u8; 64] = manifest.developer_signature.as_slice().try_into()
            .map_err(|_| "Signature must be 64 bytes".to_string())?;
        let sig = Signature::from_bytes(&sig_bytes);

        let digest = manifest.compute_canonical_digest();
        verifier_key.verify(&digest, &sig)
            .map_err(|e| format!("Manifest signature verification failed: {:?}", e))?;

        // 2. Validate Custom ObjectTypes in Range (0x1000..0xEFFF)
        for custom_type in &manifest.registered_object_types {
            if custom_type.type_id < 0x1000 || custom_type.type_id > 0xEFFF {
                return Err(format!("Invalid custom ObjectType 0x{:04X}: outside 0x1000..0xEFFF range", custom_type.type_id));
            }
            if let Some((existing_app, _)) = self.registered_schemas.get(&custom_type.type_id) {
                if existing_app != &manifest.app_id {
                    return Err(format!("ObjectType 0x{:04X} already claimed by app '{}'", custom_type.type_id, existing_app));
                }
            }
        }

        // 3. Register Schemas
        for custom_type in &manifest.registered_object_types {
            let mut s_hasher = Sha256::new();
            s_hasher.update(DOMAIN_SCHEMA);
            s_hasher.update(custom_type.schema_definition.as_bytes());
            let schema_digest = s_hasher.finalize().into();
            self.registered_schemas.insert(custom_type.type_id, (manifest.app_id.clone(), schema_digest));
        }

        // 4. Register App Namespace & Quota
        let app_ns = self.derive_app_namespace(&manifest.app_id);
        self.app_namespaces.insert(manifest.app_id.clone(), app_ns);
        self.quotas.insert(manifest.app_id.clone(), AppResourceQuota::default());
        self.installed_apps.insert(manifest.app_id.clone(), manifest);

        Ok(app_ns)
    }

    pub fn execute_sandbox(
        &mut self,
        app_id: &str,
        input_payload: &[u8],
        fuel_requested: u64,
    ) -> Result<Vec<u8>, String> {
        let manifest = self.installed_apps.get(app_id)
            .ok_or_else(|| format!("App '{}' not installed", app_id))?;
        let quota = self.quotas.get(app_id)
            .ok_or_else(|| "Quota not found".to_string())?;

        // 1. Fuel Metering Check
        if fuel_requested > quota.max_fuel {
            return Err("Execution fuel limit exceeded (infinite loop protection)".into());
        }

        // 2. Deterministic Sandbox Execution Simulation
        // In full runtime, this invokes the wasmtime engine with memory isolation
        let mut output = Vec::with_capacity(input_payload.len() + 16);
        output.extend_from_slice(b"WASM_SANDBOX_OUT:");
        output.extend_from_slice(input_payload);

        Ok(output)
    }

    pub fn uninstall_app(&mut self, app_id: &str) -> Result<(), String> {
        if let Some(manifest) = self.installed_apps.remove(app_id) {
            for custom_type in &manifest.registered_object_types {
                self.registered_schemas.remove(&custom_type.type_id);
            }
            self.app_namespaces.remove(app_id);
            self.quotas.remove(app_id);
            Ok(())
        } else {
            Err("App not installed".into())
        }
    }
}
