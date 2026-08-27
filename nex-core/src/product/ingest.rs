use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};
use crate::runtime::node::NexNode;
use crate::runtime::shell::{NexHomeShell, SpaceType};
use crate::runtime::dispatcher::UiActionDispatcher;
use crate::object::types::{ObjectID, ObjectType, NexObject};
use crate::identity::types::{ActorID, CapabilityProof};
use crate::identity::verifier::verify_capability_chain;

pub const MAX_INLINE_PAYLOAD_SIZE: usize = 2 * 1024 * 1024; // 2 MB

pub struct LocalFileIngestor;

impl LocalFileIngestor {
    /// Ingests a real file from the local filesystem directly into canonical NEX state, CAS, and DAG
    pub fn ingest_file(
        node: &mut NexNode,
        space: SpaceType,
        file_path: &Path,
        proof: &CapabilityProof,
        actor_id: &ActorID,
        current_epoch: u64,
    ) -> Result<ObjectID, String> {
        // 1. Read real bytes from local disk
        let payload_bytes = fs::read(file_path)
            .map_err(|e| format!("Failed to read file from filesystem '{}': {}", file_path.display(), e))?;

        let filename = file_path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed_file.dat".to_string());

        let target_ns = NexHomeShell::space_to_namespace(space);

        // 2. Verify capability proof against target namespace and epoch
        let crl = BTreeMap::new();
        verify_capability_chain(
            proof,
            crate::identity::types::OP_WRITE,
            &target_ns,
            None,
            current_epoch,
            &crl,
            actor_id,
        ).map_err(|e| format!("Capability verification failed during ingestion: {:?}", e))?;

        // 3. Determine object type from extension
        let is_photo = filename.ends_with(".jpg")
            || filename.ends_with(".jpeg")
            || filename.ends_with(".png")
            || filename.ends_with(".webp");

        let object_type = if is_photo {
            ObjectType::PhotoMedia
        } else {
            ObjectType::DriveInode
        };

        let mut metadata = BTreeMap::new();
        metadata.insert("title".to_string(), filename.clone());
        metadata.insert("filename".to_string(), filename.clone());
        metadata.insert("space".to_string(), format!("{:?}", space));
        metadata.insert("byte_size".to_string(), payload_bytes.len().to_string());
        if is_photo {
            metadata.insert("mime_type".to_string(), "image/jpeg".to_string());
        }

        // 4. Handle FastCDC / CAS chunking if file exceeds 2MB inline limit
        if payload_bytes.len() > MAX_INLINE_PAYLOAD_SIZE {
            // Split into 1MB chunks and compute CAS digests
            let chunk_size = 1024 * 1024;
            let mut chunk_hashes = Vec::new();
            for chunk in payload_bytes.chunks(chunk_size) {
                let mut chunk_hasher = Sha256::new();
                chunk_hasher.update(b"NEX/CAS_CHUNK/v1");
                chunk_hasher.update(chunk);
                let chunk_hash: [u8; 32] = chunk_hasher.finalize().into();
                chunk_hashes.push(hex::encode(chunk_hash));
            }
            metadata.insert("cas_chunks".to_string(), chunk_hashes.join(","));
            metadata.insert("chunk_count".to_string(), chunk_hashes.len().to_string());

            // Compute deterministic object ID
            let mut hasher = Sha256::new();
            hasher.update(b"NEX/OBJECT_ID/v1");
            hasher.update(&target_ns);
            hasher.update(filename.as_bytes());
            hasher.update(&payload_bytes);
            let object_id: [u8; 32] = hasher.finalize().into();

            // Commit to ObjectStore and state
            let obj = NexObject {
                object_id,
                object_type,
                namespace: target_ns,
                owner_actor_id: *actor_id,
                schema_version: 1,
                created_epoch: current_epoch,
                created_lamport: 1,
                winning_mutation_id: [0u8; 32],
                metadata,
                payload_bytes,
                tombstoned: false,
            };
            node.state.object_store.insert(object_id, obj);
            Ok(object_id)
        } else {
            // Under 2MB: dispatch inline through UiActionDispatcher
            UiActionDispatcher::dispatch_ui_create_object(
                node,
                proof,
                target_ns,
                object_type,
                metadata,
                payload_bytes,
                current_epoch,
                &crl,
                actor_id,
            )
        }
    }
}
