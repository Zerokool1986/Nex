use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, OnceLock};
use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use crate::runtime::node::NexNode;
use crate::runtime::mobile::DevicePowerState;

pub const NEX_ABI_VERSION_1: u32 = 0x00010000;
pub const NEX_HANDLE_MAGIC: u32 = 0x4E455848; // "NEXH"

// Error codes
pub const NEX_SUCCESS: i32 = 0;
pub const NEX_ERR_INVALID_HANDLE: i32 = -4001;
pub const NEX_ERR_UNAUTHORIZED: i32 = -4010;
pub const NEX_ERR_OBJECT_NOT_FOUND: i32 = -4040;
pub const NEX_ERR_OBJECT_TOMBSTONED: i32 = -4100;
pub const NEX_ERR_PAYLOAD_TOO_LARGE: i32 = -4130;
pub const NEX_ERR_CAS_CORRUPTION: i32 = -4140;
pub const NEX_ERR_SYNC_DEFERRED: i32 = -4290;
pub const NEX_ERR_STORAGE_EXHAUSTED: i32 = -5070;
pub const NEX_ERR_INTERNAL_ERROR: i32 = -5000;

pub type EventCallback = Box<dyn Fn(&[u8]) + Send + Sync + 'static>;

pub struct EventSubscription {
    pub id: u64,
    pub topic: String,
    pub callback: EventCallback,
}

pub struct RuntimeInstance {
    pub magic: u32,
    pub handle_id: u64,
    pub node: Mutex<NexNode>,
    pub data_dir: PathBuf,
    pub is_running: AtomicBool,
    pub power_state: Mutex<DevicePowerState>,
    pub subscriptions: RwLock<HashMap<u64, Arc<EventSubscription>>>,
    pub next_sub_id: AtomicU64,
    pub shutdown_signal: AtomicBool,
}

impl RuntimeInstance {
    pub fn new(handle_id: u64, data_dir: PathBuf, master_seed: [u8; 32]) -> Result<Self, i32> {
        let signing_key = SigningKey::from_bytes(&master_seed);
        let node = NexNode::new(data_dir.clone(), signing_key);

        Ok(Self {
            magic: NEX_HANDLE_MAGIC,
            handle_id,
            node: Mutex::new(node),
            data_dir,
            is_running: AtomicBool::new(false),
            power_state: Mutex::new(DevicePowerState::Active),
            subscriptions: RwLock::new(HashMap::new()),
            next_sub_id: AtomicU64::new(1),
            shutdown_signal: AtomicBool::new(false),
        })
    }

    pub fn start(&self) -> i32 {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return NEX_SUCCESS; // already running
        }
        let mut node = match self.node.lock() {
            Ok(g) => g,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };
        match node.start() {
            Ok(_) => NEX_SUCCESS,
            Err(_) => {
                self.is_running.store(false, Ordering::SeqCst);
                NEX_ERR_INTERNAL_ERROR
            }
        }
    }

    pub fn stop(&self) -> i32 {
        self.shutdown_signal.store(true, Ordering::SeqCst);
        if !self.is_running.swap(false, Ordering::SeqCst) {
            return NEX_SUCCESS; // already stopped
        }
        let mut node = match self.node.lock() {
            Ok(g) => g,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };
        match node.stop() {
            Ok(_) => NEX_SUCCESS,
            Err(_) => NEX_ERR_INTERNAL_ERROR,
        }
    }

    pub fn subscribe<F>(&self, topic: &str, callback: F) -> u64
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        let id = self.next_sub_id.fetch_add(1, Ordering::SeqCst);
        let sub = Arc::new(EventSubscription {
            id,
            topic: topic.to_string(),
            callback: Box::new(callback),
        });
        if let Ok(mut map) = self.subscriptions.write() {
            map.insert(id, sub);
        }
        id
    }

    pub fn unsubscribe(&self, sub_id: u64) -> bool {
        if let Ok(mut map) = self.subscriptions.write() {
            map.remove(&sub_id).is_some()
        } else {
            false
        }
    }

    pub fn emit_event(&self, topic: &str, payload: &[u8]) {
        if self.shutdown_signal.load(Ordering::SeqCst) {
            return;
        }
        if let Ok(map) = self.subscriptions.read() {
            for sub in map.values() {
                if sub.topic == topic || sub.topic == "*" {
                    (sub.callback)(payload);
                }
            }
        }
    }
}

// Global handle registry ensuring true double-free immunity
static GLOBAL_REGISTRY: OnceLock<RwLock<HashMap<u64, Arc<RuntimeInstance>>>> = OnceLock::new();
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1001);

fn get_registry() -> &'static RwLock<HashMap<u64, Arc<RuntimeInstance>>> {
    GLOBAL_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct HandleRegistry;

impl HandleRegistry {
    pub fn register(instance: RuntimeInstance) -> u64 {
        let handle_id = instance.handle_id;
        let arc = Arc::new(instance);
        let registry = get_registry();
        if let Ok(mut map) = registry.write() {
            map.insert(handle_id, arc);
        }
        handle_id
    }

    pub fn get(handle_id: u64) -> Option<Arc<RuntimeInstance>> {
        let registry = get_registry();
        let map = registry.read().ok()?;
        map.get(&handle_id).cloned()
    }

    pub fn remove(handle_id: u64) -> Option<Arc<RuntimeInstance>> {
        let registry = get_registry();
        let mut map = registry.write().ok()?;
        if let Some(instance) = map.remove(&handle_id) {
            instance.stop();
            Some(instance)
        } else {
            None
        }
    }

    pub fn allocate_id() -> u64 {
        NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst)
    }

    pub fn active_count() -> usize {
        get_registry().read().map(|r| r.len()).unwrap_or(0)
    }
}
