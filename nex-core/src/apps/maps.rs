use std::collections::{BTreeMap, BTreeSet};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::runtime::node::NexNode;
use crate::object::types::{NamespaceID, ObjectType, NexObject};
use crate::api::NexAppApi;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TileCoordinate {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

impl TileCoordinate {
    pub fn new(zoom: u8, x: u32, y: u32) -> Self {
        Self { zoom, x, y }
    }

    pub fn cas_key(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/MAPS/TILE/v1");
        hasher.update(&[self.zoom]);
        hasher.update(&self.x.to_be_bytes());
        hasher.update(&self.y.to_be_bytes());
        hasher.finalize().into()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub altitude_m: Option<f64>,
    pub category: String,
    pub created_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpsPoint {
    pub lat: f64,
    pub lon: f64,
    pub altitude_m: f64,
    pub timestamp_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpsTrackLog {
    pub track_id: String,
    pub name: String,
    pub points: Vec<GpsPoint>,
    pub total_distance_meters: f64,
}

impl GpsTrackLog {
    pub fn new(track_id: &str, name: &str) -> Self {
        Self {
            track_id: track_id.to_string(),
            name: name.to_string(),
            points: Vec::new(),
            total_distance_meters: 0.0,
        }
    }

    pub fn append_point(&mut self, lat: f64, lon: f64, altitude_m: f64, timestamp: u64) {
        if let Some(last) = self.points.last() {
            let d = calculate_haversine_distance(last.lat, last.lon, lat, lon);
            self.total_distance_meters += d;
        }
        self.points.push(GpsPoint {
            lat,
            lon,
            altitude_m,
            timestamp_epoch: timestamp,
        });
    }
}

fn calculate_haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0; // Earth radius in meters
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_phi = (lat2 - lat1).to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();

    let a = (delta_phi / 2.0).sin().powi(2)
        + phi1.cos() * phi2.cos() * (delta_lambda / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

pub struct NexMapsService;

impl NexMapsService {
    pub const MAPS_NAMESPACE: NamespaceID = [0xAA; 32];

    pub fn store_vector_tile(node: &mut NexNode, coord: TileCoordinate, tile_data: Vec<u8>) -> Result<[u8; 32], String> {
        let chunk_digest = node.storage.cas.put_chunk(&tile_data);
        let mut meta = BTreeMap::new();
        meta.insert("zoom".to_string(), coord.zoom.to_string());
        meta.insert("x".to_string(), coord.x.to_string());
        meta.insert("y".to_string(), coord.y.to_string());
        meta.insert("tile_hash".to_string(), hex::encode(chunk_digest));

        node.create_object(Self::MAPS_NAMESPACE, ObjectType::Synthetic(11), meta, tile_data)
            .map_err(|e| format!("{:?}", e))
    }

    pub fn save_waypoint(node: &mut NexNode, wp: &Waypoint) -> Result<[u8; 32], String> {
        let mut meta = BTreeMap::new();
        meta.insert("wp_id".to_string(), wp.id.clone());
        meta.insert("name".to_string(), wp.name.clone());
        meta.insert("lat".to_string(), wp.lat.to_string());
        meta.insert("lon".to_string(), wp.lon.to_string());
        meta.insert("category".to_string(), wp.category.clone());

        let payload = serde_json::to_vec(wp).map_err(|e| format!("{:?}", e))?;
        node.create_object(Self::MAPS_NAMESPACE, ObjectType::Synthetic(10), meta, payload)
            .map_err(|e| format!("{:?}", e))
    }

    pub fn save_track_log(node: &mut NexNode, track: &GpsTrackLog) -> Result<[u8; 32], String> {
        let mut meta = BTreeMap::new();
        meta.insert("track_id".to_string(), track.track_id.clone());
        meta.insert("name".to_string(), track.name.clone());
        meta.insert("points_count".to_string(), track.points.len().to_string());
        meta.insert("distance_m".to_string(), track.total_distance_meters.to_string());

        let payload = serde_json::to_vec(track).map_err(|e| format!("{:?}", e))?;
        node.create_object(Self::MAPS_NAMESPACE, ObjectType::Synthetic(12), meta, payload)
            .map_err(|e| format!("{:?}", e))
    }
}
