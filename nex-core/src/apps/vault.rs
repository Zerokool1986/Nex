use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NamespaceID, ObjectType};
use crate::api::NexAppApi;
use crate::identity::types::CapabilityProof;

pub const DOMAIN_VAULT_ITEM: &[u8] = b"NEX/VAULT/ITEM/v1";
pub const DOMAIN_VAULT_CIPHER: &[u8] = b"NEX/VAULT/CIPHER/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultCategory {
    Login,
    SecureNote,
    TotpSeed,
    CryptoKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultItem {
    pub item_id: ObjectID,
    pub namespace: NamespaceID,
    pub title: String,
    pub category: VaultCategory,
    pub ciphertext: Vec<u8>,
    pub updated_epoch: u64,
}

pub struct NexVaultEngine<A: NexAppApi> {
    pub namespace_id: NamespaceID,
    pub api: A,
    pub items: BTreeMap<ObjectID, VaultItem>,
}

impl<A: NexAppApi> NexVaultEngine<A> {
    pub fn new(namespace_id: NamespaceID, api: A) -> Self {
        Self {
            namespace_id,
            api,
            items: BTreeMap::new(),
        }
    }

    pub fn encrypt_secret(plaintext: &[u8], master_key: &[u8; 32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(plaintext.len() + 32);
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_VAULT_CIPHER);
        hasher.update(master_key);
        let mask: [u8; 32] = hasher.finalize().into();

        for (i, &b) in plaintext.iter().enumerate() {
            out.push(b ^ mask[i % 32]);
        }

        let mut mac_hasher = Sha256::new();
        mac_hasher.update(b"NEX/VAULT/MAC/v1");
        mac_hasher.update(master_key);
        mac_hasher.update(&out);
        let mac: [u8; 32] = mac_hasher.finalize().into();
        out.extend_from_slice(&mac);
        out
    }

    pub fn decrypt_secret(ciphertext: &[u8], master_key: &[u8; 32]) -> Result<Vec<u8>, String> {
        if ciphertext.len() < 32 {
            return Err("Ciphertext too short".into());
        }
        let (cipher_body, mac) = ciphertext.split_at(ciphertext.len() - 32);
        let mut mac_hasher = Sha256::new();
        mac_hasher.update(b"NEX/VAULT/MAC/v1");
        mac_hasher.update(master_key);
        mac_hasher.update(cipher_body);
        let expected_mac = mac_hasher.finalize();
        if expected_mac.as_slice() != mac {
            return Err("MAC validation failure: secret is corrupted or master key is incorrect".into());
        }

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_VAULT_CIPHER);
        hasher.update(master_key);
        let mask: [u8; 32] = hasher.finalize().into();

        let mut out = Vec::with_capacity(cipher_body.len());
        for (i, &b) in cipher_body.iter().enumerate() {
            out.push(b ^ mask[i % 32]);
        }
        Ok(out)
    }

    pub fn store_item(
        &mut self,
        title: &str,
        category: VaultCategory,
        secret_payload: &[u8],
        master_key: &[u8; 32],
        proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let ciphertext = Self::encrypt_secret(secret_payload, master_key);

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_VAULT_ITEM);
        hasher.update(&self.namespace_id);
        hasher.update(title.as_bytes());
        let item = VaultItem {
            item_id: [0u8; 32],
            namespace: self.namespace_id,
            title: title.to_string(),
            category,
            ciphertext,
            updated_epoch: 0,
        };

        let encoded = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), title.to_string());
        meta.insert("category".to_string(), format!("{:?}", category));

        let _ = proof;
        let obj_id = self.api.create_object(
            self.namespace_id,
            ObjectType::VaultItem,
            meta,
            encoded,
        ).map_err(|e| format!("{:?}", e))?;

        let mut final_item = item;
        final_item.item_id = obj_id;
        self.items.insert(obj_id, final_item);
        Ok(obj_id)
    }

    pub fn read_item(&self, item_id: &ObjectID, master_key: &[u8; 32]) -> Result<(VaultItem, Vec<u8>), String> {
        let obj = self.api.read_object(item_id).map_err(|e| format!("{:?}", e))?;
        if obj.tombstoned {
            return Err("Vault item is tombstoned".into());
        }
        let item: VaultItem = serde_json::from_slice(&obj.payload_bytes).map_err(|e| e.to_string())?;
        let decrypted = Self::decrypt_secret(&item.ciphertext, master_key)?;
        Ok((item, decrypted))
    }

    pub fn delete_item(&mut self, item_id: ObjectID, proof: Option<CapabilityProof>) -> Result<(), String> {
        self.api.delete_object(item_id, proof).map_err(|e| format!("{:?}", e))?;
        self.items.remove(&item_id);
        Ok(())
    }

    pub fn list_items(&self) -> Vec<VaultItem> {
        self.items.values().cloned().collect()
    }
}
