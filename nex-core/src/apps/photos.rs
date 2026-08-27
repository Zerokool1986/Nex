use std::collections::{BTreeMap, BTreeSet};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NamespaceID, ObjectType, NexObject};
use crate::api::{NexAppApi, CoreRuntimeError};
use crate::apps::drive::CasChunkStore;
use crate::identity::types::{ActorID, CapabilityProof, OP_REGISTER_LWW, OP_OBJECT_TOMBSTONE, OP_ALL};

pub const DOMAIN_PHOTO_MEDIA: &[u8] = b"NEX/PHOTO/MEDIA/v1";
pub const DOMAIN_PHOTO_ALBUM: &[u8] = b"NEX/PHOTO/ALBUM/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub width: u32,
    pub height: u32,
    pub capture_timestamp: u64,
    pub camera_make: String,
    pub camera_model: String,
    pub lens_model: Option<String>,
    pub iso: Option<u32>,
    pub exposure_time: Option<String>,
    pub f_number: Option<f32>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoMedia {
    pub media_id: ObjectID,
    pub namespace: NamespaceID,
    pub original_filename: String,
    pub mime_type: String,
    pub raw_content_root: [u8; 32],
    pub raw_chunk_digests: Vec<[u8; 32]>,
    pub raw_byte_size: u64,
    pub preview_content_root: [u8; 32],
    pub preview_chunk_digests: Vec<[u8; 32]>,
    pub thumbnail_digest: [u8; 32],
    pub metadata: MediaMetadata,
    pub created_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoAlbum {
    pub album_id: ObjectID,
    pub namespace: NamespaceID,
    pub title: String,
    pub description: String,
    pub cover_media_id: Option<ObjectID>,
    pub media_items: Vec<ObjectID>,
    pub is_smart_album: bool,
}

pub struct NexPhotosEngine<A: NexAppApi> {
    pub namespace_id: NamespaceID,
    pub local_actor_id: ActorID,
    pub api: A,
    pub cas: CasChunkStore,
    pub photos: BTreeMap<ObjectID, PhotoMedia>,
    pub albums: BTreeMap<ObjectID, PhotoAlbum>,
}

impl<A: NexAppApi> NexPhotosEngine<A> {
    pub fn new(namespace_id: NamespaceID, local_actor_id: ActorID, api: A, cas: CasChunkStore) -> Self {
        Self {
            namespace_id,
            local_actor_id,
            api,
            cas,
            photos: BTreeMap::new(),
            albums: BTreeMap::new(),
        }
    }

    pub fn import_photo(
        &mut self,
        filename: &str,
        mime_type: &str,
        raw_data: &[u8],
        metadata: MediaMetadata,
    ) -> Result<ObjectID, String> {
        // 1. Store Raw Master in CAS
        let (raw_content_root, raw_chunk_digests) = self.cas.store_file(raw_data);

        // 2. Generate and store 2048px preview simulation in CAS
        let preview_sim = if raw_data.len() > 1024 { &raw_data[..1024] } else { raw_data };
        let (preview_content_root, preview_chunk_digests) = self.cas.store_file(preview_sim);

        // 3. Generate and store 256px thumbnail simulation in CAS
        let thumb_sim = if raw_data.len() > 256 { &raw_data[..256] } else { raw_data };
        let thumbnail_digest = self.cas.put_chunk(thumb_sim);

        // 4. Derive deterministic MediaID
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_PHOTO_MEDIA);
        hasher.update(&self.namespace_id);
        hasher.update(filename.as_bytes());
        hasher.update(&raw_content_root);
        let media_id: ObjectID = hasher.finalize().into();

        let photo = PhotoMedia {
            media_id,
            namespace: self.namespace_id,
            original_filename: filename.to_string(),
            mime_type: mime_type.to_string(),
            raw_content_root,
            raw_chunk_digests,
            raw_byte_size: raw_data.len() as u64,
            preview_content_root,
            preview_chunk_digests,
            thumbnail_digest,
            metadata,
            created_epoch: 0,
        };

        let encoded = serde_json::to_vec(&photo).map_err(|e| e.to_string())?;
        let mut meta = BTreeMap::new();
        meta.insert("filename".into(), filename.to_string());
        meta.insert("mime".into(), mime_type.to_string());

        self.api.create_object(
            self.namespace_id,
            ObjectType::PhotoMedia,
            meta,
            encoded,
        ).map_err(|e| format!("{:?}", e))?;

        self.photos.insert(media_id, photo);
        Ok(media_id)
    }

    pub fn create_album(
        &mut self,
        title: &str,
        description: &str,
        initial_photos: Vec<ObjectID>,
    ) -> Result<ObjectID, String> {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_PHOTO_ALBUM);
        hasher.update(&self.namespace_id);
        hasher.update(title.as_bytes());
        let album_id: ObjectID = hasher.finalize().into();

        let album = PhotoAlbum {
            album_id,
            namespace: self.namespace_id,
            title: title.to_string(),
            description: description.to_string(),
            cover_media_id: initial_photos.first().copied(),
            media_items: initial_photos,
            is_smart_album: false,
        };

        let encoded = serde_json::to_vec(&album).map_err(|e| e.to_string())?;
        let mut meta = BTreeMap::new();
        meta.insert("title".into(), title.to_string());

        self.api.create_object(
            self.namespace_id,
            ObjectType::PhotoAlbum,
            meta,
            encoded,
        ).map_err(|e| format!("{:?}", e))?;

        self.albums.insert(album_id, album);
        Ok(album_id)
    }

    pub fn add_photo_to_album(&mut self, album_id: ObjectID, photo_id: ObjectID) -> Result<(), String> {
        let album = self.albums.get_mut(&album_id)
            .ok_or_else(|| "Album not found".to_string())?;
        if !album.media_items.contains(&photo_id) {
            album.media_items.push(photo_id);
        }
        Ok(())
    }

    pub fn compute_album_merkle_digest(&self, album_id: &ObjectID) -> Result<[u8; 32], String> {
        let album = self.albums.get(album_id)
            .ok_or_else(|| "Album not found".to_string())?;

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_PHOTO_ALBUM);
        hasher.update(&album.album_id);
        for item in &album.media_items {
            hasher.update(item);
        }
        Ok(hasher.finalize().into())
    }

    pub fn get_redacted_media_view(&self, media_id: &ObjectID, allow_gps: bool) -> Result<PhotoMedia, String> {
        let photo = self.photos.get(media_id)
            .ok_or_else(|| "Photo not found".to_string())?;

        let mut view = photo.clone();
        if !allow_gps {
            view.metadata.gps_latitude = None;
            view.metadata.gps_longitude = None;
        }
        Ok(view)
    }

    pub fn delete_photo(&mut self, media_id: ObjectID, proof: Option<CapabilityProof>) -> Result<(), String> {
        self.api.delete_object(media_id, proof).map_err(|e| format!("{:?}", e))?;
        self.photos.remove(&media_id);
        // Remove from all albums
        for album in self.albums.values_mut() {
            album.media_items.retain(|id| *id != media_id);
        }
        Ok(())
    }
}
