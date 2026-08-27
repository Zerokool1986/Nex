use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NamespaceID, ObjectType};
use crate::api::NexAppApi;
use crate::identity::types::CapabilityProof;
use crate::model::{Mutation, MutationBody, CrdtPayload};
use crate::hash::hash_mutation_body;

pub const DOMAIN_DRIVE_INODE: &[u8] = b"NEX/DRIVE/INODE/v1";
pub const DOMAIN_DRIVE_DIR:   &[u8] = b"NEX/DRIVE/DIR/v1";
pub const DOMAIN_CAS_CHUNK:   &[u8] = b"NEX/CAS_CHUNK/v1";

pub const CHUNK_SIZE_2MB: usize = 2 * 1024 * 1024; // 2MB

pub fn normalize_vpath(vpath: &str) -> String {
    let replaced = vpath.replace('\\', "/");
    let segments: Vec<&str> = replaced
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

pub fn derive_drive_object_id(namespace: &NamespaceID, vpath: &str) -> ObjectID {
    let norm = normalize_vpath(vpath);
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_DRIVE_INODE);
    hasher.update(namespace);
    hasher.update(norm.as_bytes());
    hasher.finalize().into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveInode {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub mime_type: String,
    pub content_root: [u8; 32],
    pub chunk_digests: Vec<[u8; 32]>,
    pub modified_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveFolder {
    pub path: String,
    pub name: String,
    pub entries: BTreeMap<String, (ObjectID, ObjectType)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFileEntry {
    pub path: String,
    pub content_hash: [u8; 32],
    pub size_bytes: u64,
    pub mime_type: String,
    pub epoch: u64,
}

#[derive(Debug, Default, Clone)]
pub struct CasChunkStore {
    pub chunks: BTreeMap<[u8; 32], Vec<u8>>,
}

impl CasChunkStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put_chunk(&mut self, data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_CAS_CHUNK);
        hasher.update(data);
        let digest: [u8; 32] = hasher.finalize().into();
        self.chunks.entry(digest).or_insert_with(|| data.to_vec());
        digest
    }

    pub fn get_chunk(&self, digest: &[u8; 32]) -> Option<&Vec<u8>> {
        self.chunks.get(digest)
    }

    pub fn has_chunk(&self, digest: &[u8; 32]) -> bool {
        self.chunks.contains_key(digest)
    }

    pub fn store_file(&mut self, content: &[u8]) -> ([u8; 32], Vec<[u8; 32]>) {
        if content.is_empty() {
            let digest = self.put_chunk(&[]);
            return (digest, vec![digest]);
        }

        let mut digests = Vec::new();
        for chunk in content.chunks(CHUNK_SIZE_2MB) {
            let digest = self.put_chunk(chunk);
            digests.push(digest);
        }

        let content_root = Self::compute_merkle_root(&digests);
        (content_root, digests)
    }

    pub fn assemble_file(&self, digests: &[[u8; 32]]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        for d in digests {
            let chunk = self.get_chunk(d).ok_or_else(|| format!("Missing chunk: {:?}", d))?;
            out.extend_from_slice(chunk);
        }
        Ok(out)
    }

    pub fn verify_chunk(&self, digest: &[u8; 32]) -> bool {
        if let Some(data) = self.chunks.get(digest) {
            let mut hasher = Sha256::new();
            hasher.update(DOMAIN_CAS_CHUNK);
            hasher.update(data);
            let calculated: [u8; 32] = hasher.finalize().into();
            calculated == *digest
        } else {
            false
        }
    }

    pub fn heal_chunk(&mut self, digest: [u8; 32], fresh_valid_data: &[u8]) -> Result<(), String> {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_CAS_CHUNK);
        hasher.update(fresh_valid_data);
        let calculated: [u8; 32] = hasher.finalize().into();
        if calculated != digest {
            return Err("HealDataDigestMismatch: candidate data does not match target digest".into());
        }
        self.chunks.insert(digest, fresh_valid_data.to_vec());
        Ok(())
    }

    pub fn sweep_unreferenced(&mut self, active_references: &std::collections::BTreeSet<[u8; 32]>) -> usize {
        let mut to_remove = Vec::new();
        for key in self.chunks.keys() {
            if !active_references.contains(key) {
                to_remove.push(*key);
            }
        }
        let count = to_remove.len();
        for key in to_remove {
            self.chunks.remove(&key);
        }
        count
    }

    pub fn check_storage_quota(&self, quota_bytes: usize, incoming_bytes: usize) -> Result<(), String> {
        let current_bytes: usize = self.chunks.values().map(|c| c.len()).sum();
        if current_bytes + incoming_bytes > quota_bytes {
            return Err("StorageExhausted: capacity ceiling exceeded".into());
        }
        Ok(())
    }

    pub fn compute_merkle_root(digests: &[[u8; 32]]) -> [u8; 32] {
        if digests.is_empty() {
            return [0u8; 32];
        }
        if digests.len() == 1 {
            return digests[0];
        }

        let mut current_layer = digests.to_vec();
        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();
            for chunk in current_layer.chunks(2) {
                if chunk.len() == 2 {
                    let mut hasher = Sha256::new();
                    hasher.update(b"NEX/CAS_TREE/v1");
                    hasher.update(&chunk[0]);
                    hasher.update(&chunk[1]);
                    next_layer.push(hasher.finalize().into());
                } else {
                    next_layer.push(chunk[0]);
                }
            }
            current_layer = next_layer;
        }
        current_layer[0]
    }
}

#[derive(Debug, Clone)]
pub struct DriveEngine {
    pub namespace_id: NamespaceID,
    pub cas: CasChunkStore,
    pub directories: BTreeMap<String, DriveFolder>,
    pub files: BTreeMap<String, DriveFileEntry>,
}

impl DriveEngine {
    pub fn new(namespace_id: NamespaceID) -> Self {
        let mut engine = Self {
            namespace_id,
            cas: CasChunkStore::new(),
            directories: BTreeMap::new(),
            files: BTreeMap::new(),
        };
        let root = DriveFolder {
            path: "/".to_string(),
            name: "".to_string(),
            entries: BTreeMap::new(),
        };
        engine.directories.insert("/".to_string(), root);
        engine
    }

    pub fn create_directory(&mut self, path: &str, _epoch: u64) {
        let name = path.split('/').last().unwrap_or("").to_string();
        let folder = DriveFolder {
            path: path.to_string(),
            name,
            entries: BTreeMap::new(),
        };
        self.directories.insert(path.to_string(), folder);
    }

    pub fn create_file(
        &mut self,
        path: &str,
        content_hash: [u8; 32],
        size_bytes: u64,
        mime_type: &str,
        epoch: u64,
    ) -> Mutation {
        let obj_id = derive_drive_object_id(&self.namespace_id, path);
        self.files.insert(
            path.to_string(),
            DriveFileEntry {
                path: path.to_string(),
                content_hash,
                size_bytes,
                mime_type: mime_type.to_string(),
                epoch,
            },
        );
        let body = MutationBody {
            author: [0u8; 32],
            parents: vec![],
            lamport: epoch,
            epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: obj_id, value: content_hash.to_vec() },
        };
        Mutation {
            id: hash_mutation_body(&body),
            body,
        }
    }

    pub fn delete_file(&mut self, path: &str, epoch: u64) -> Mutation {
        let obj_id = derive_drive_object_id(&self.namespace_id, path);
        self.files.remove(path);
        let body = MutationBody {
            author: [0u8; 32],
            parents: vec![],
            lamport: epoch,
            epoch,
            is_resurrect: false,
            payload: CrdtPayload::Tombstone { id: obj_id },
        };
        Mutation {
            id: hash_mutation_body(&body),
            body,
        }
    }
}

pub struct NexDriveEngine<A: NexAppApi> {
    pub namespace_id: NamespaceID,
    pub api: A,
    pub cas: CasChunkStore,
    pub directories: BTreeMap<String, DriveFolder>,
}

impl<A: NexAppApi> NexDriveEngine<A> {
    pub fn new(namespace_id: NamespaceID, api: A) -> Self {
        let mut engine = Self {
            namespace_id,
            api,
            cas: CasChunkStore::new(),
            directories: BTreeMap::new(),
        };
        let root = DriveFolder {
            path: "/".to_string(),
            name: "".to_string(),
            entries: BTreeMap::new(),
        };
        engine.directories.insert("/".to_string(), root);
        engine
    }

    pub fn sanitize_path(raw_path: &str) -> Result<String, String> {
        if raw_path.contains("..") || raw_path.contains('\0') {
            return Err("Path traversal (..) or null byte forbidden".into());
        }
        let replaced = raw_path.replace('\\', "/");
        let segments: Vec<&str> = replaced
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect();
        if segments.is_empty() {
            return Ok("/".to_string());
        }
        Ok(format!("/{}", segments.join("/")))
    }

    pub fn upload_file(
        &mut self,
        vpath: &str,
        mime_type: &str,
        content: &[u8],
        _proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let clean_path = Self::sanitize_path(vpath)?;
        let name = clean_path.split('/').filter(|s| !s.is_empty()).last().unwrap_or("file").to_string();

        let (content_root, chunk_digests) = self.cas.store_file(content);
        let inode = DriveInode {
            path: clean_path.clone(),
            name: name.clone(),
            size_bytes: content.len() as u64,
            mime_type: mime_type.to_string(),
            content_root,
            chunk_digests,
            modified_epoch: 1,
        };

        let payload = serde_json::to_vec(&inode).map_err(|e| e.to_string())?;
        let metadata = BTreeMap::from([
            ("path".to_string(), clean_path.clone()),
            ("mime_type".to_string(), mime_type.to_string()),
            ("size_bytes".to_string(), content.len().to_string()),
        ]);

        let obj_id = self.api.create_object(
            self.namespace_id,
            ObjectType::DriveInode,
            metadata,
            payload,
        ).map_err(|e| format!("{:?}", e))?;

        let parent_dir = clean_path.rsplit_once('/').map(|(p, _)| if p.is_empty() { "/" } else { p }).unwrap_or("/");
        self.directories.entry(parent_dir.to_string())
            .or_insert_with(|| DriveFolder {
                path: parent_dir.to_string(),
                name: parent_dir.to_string(),
                entries: BTreeMap::new(),
            })
            .entries
            .insert(name, (obj_id, ObjectType::DriveInode));

        Ok(obj_id)
    }

    pub fn download_file(&self, object_id: &ObjectID) -> Result<Vec<u8>, String> {
        let obj = self.api.read_object(object_id).map_err(|e| format!("{:?}", e))?;
        let inode: DriveInode = serde_json::from_slice(&obj.payload_bytes).map_err(|e| e.to_string())?;
        self.cas.assemble_file(&inode.chunk_digests)
    }

    pub fn delete_file(
        &mut self,
        vpath: &str,
        object_id: ObjectID,
        proof: Option<CapabilityProof>,
    ) -> Result<(), String> {
        self.api.delete_object(object_id, proof).map_err(|e| format!("{:?}", e))?;
        let clean_path = Self::sanitize_path(vpath)?;
        let name = clean_path.split('/').filter(|s| !s.is_empty()).last().unwrap_or("");
        let parent_dir = clean_path.rsplit_once('/').map(|(p, _)| if p.is_empty() { "/" } else { p }).unwrap_or("/");
        if let Some(folder) = self.directories.get_mut(parent_dir) {
            folder.entries.remove(name);
        }
        Ok(())
    }

    pub fn compute_directory_merkle_digest(&self, dir_path: &str) -> [u8; 32] {
        let clean_path = Self::sanitize_path(dir_path).unwrap_or_else(|_| "/".into());
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_DRIVE_DIR);
        hasher.update(clean_path.as_bytes());

        if let Some(dir) = self.directories.get(&clean_path) {
            for (name, (obj_id, obj_type)) in &dir.entries {
                hasher.update(name.as_bytes());
                hasher.update(&obj_type.as_u16().to_le_bytes());
                hasher.update(obj_id);
            }
        }
        hasher.finalize().into()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DriveSyncDelta {
    pub added_files: usize,
    pub updated_files: usize,
    pub deleted_files: usize,
    pub synced_bytes: u64,
}

pub struct DriveSyncWatcher<A: NexAppApi> {
    pub engine: NexDriveEngine<A>,
    pub local_manifest: BTreeMap<String, ([u8; 32], u64)>,
}

impl<A: NexAppApi> DriveSyncWatcher<A> {
    pub fn new(engine: NexDriveEngine<A>) -> Self {
        Self {
            engine,
            local_manifest: BTreeMap::new(),
        }
    }

    pub fn scan_and_sync(&mut self, local_root: &std::path::Path) -> Result<DriveSyncDelta, String> {
        let mut delta = DriveSyncDelta::default();
        if !local_root.exists() {
            return Ok(delta);
        }

        let mut discovered = BTreeMap::new();
        self.traverse_dir(local_root, local_root, &mut discovered)?;

        for (vpath, (content, size)) in &discovered {
            let mut hasher = Sha256::new();
            hasher.update(DOMAIN_CAS_CHUNK);
            hasher.update(content);
            let content_hash: [u8; 32] = hasher.finalize().into();

            if let Some((prev_hash, _)) = self.local_manifest.get(vpath) {
                if *prev_hash != content_hash {
                    let _ = self.engine.upload_file(vpath, "application/octet-stream", content, None)?;
                    self.local_manifest.insert(vpath.clone(), (content_hash, *size));
                    delta.updated_files += 1;
                    delta.synced_bytes += size;
                }
            } else {
                let _ = self.engine.upload_file(vpath, "application/octet-stream", content, None)?;
                self.local_manifest.insert(vpath.clone(), (content_hash, *size));
                delta.added_files += 1;
                delta.synced_bytes += size;
            }
        }

        Ok(delta)
    }

    fn traverse_dir(
        &self,
        base: &std::path::Path,
        current: &std::path::Path,
        out: &mut BTreeMap<String, (Vec<u8>, u64)>,
    ) -> Result<(), String> {
        if current.is_dir() {
            let entries = std::fs::read_dir(current).map_err(|e| e.to_string())?;
            for entry in entries {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                if path.is_dir() {
                    self.traverse_dir(base, &path, out)?;
                } else if path.is_file() {
                    let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
                    let vpath = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
                    let content = std::fs::read(&path).map_err(|e| e.to_string())?;
                    let size = content.len() as u64;
                    out.insert(vpath, (content, size));
                }
            }
        }
        Ok(())
    }
}
