use serde::{Deserialize, Serialize};
use crate::runtime::node::NexNode;
use crate::object::types::NamespaceID;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SpaceType {
    Personal,
    Family,
    Work,
    Community,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeFeedItem {
    pub id: [u8; 16],
    pub title: String,
    pub body: String,
    pub space: SpaceType,
    pub timestamp_epoch: u64,
    pub action_route: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeSummary {
    pub active_space: SpaceType,
    pub total_objects_in_space: usize,
    pub is_synchronized: bool,
    pub quick_actions: Vec<String>,
}

pub struct NexHomeShell {
    pub active_space: SpaceType,
}

impl NexHomeShell {
    pub fn new() -> Self {
        Self {
            active_space: SpaceType::Personal,
        }
    }

    pub fn switch_space(&mut self, space: SpaceType) {
        self.active_space = space;
    }

    pub fn space_to_namespace(space: SpaceType) -> NamespaceID {
        let mut ns = [0u8; 32];
        match space {
            SpaceType::Personal => ns[0] = 0x01,
            SpaceType::Family => ns[0] = 0x02,
            SpaceType::Work => ns[0] = 0x03,
            SpaceType::Community => ns[0] = 0x04,
            SpaceType::Project => ns[0] = 0x05,
        }
        ns
    }

    pub fn generate_home_summary(&self, node: &NexNode) -> HomeSummary {
        let target_ns = Self::space_to_namespace(self.active_space);
        let objects_count = node.state.object_store
            .values()
            .filter(|o| o.namespace == target_ns && !o.tombstoned)
            .count();

        HomeSummary {
            active_space: self.active_space,
            total_objects_in_space: objects_count,
            is_synchronized: true,
            quick_actions: vec![
                "Message".to_string(),
                "Share".to_string(),
                "Camera".to_string(),
                "PairDevice".to_string(),
            ],
        }
    }

    pub fn recent_activity_for_space(&self, node: &NexNode, space: SpaceType) -> Vec<HomeFeedItem> {
        let target_ns = Self::space_to_namespace(space);
        let mut items = Vec::new();

        for (id, obj) in &node.state.object_store {
            if obj.namespace == target_ns && !obj.tombstoned {
                let mut feed_id = [0u8; 16];
                feed_id.copy_from_slice(&id[..16]);
                items.push(HomeFeedItem {
                    id: feed_id,
                    title: format!("Object {:?}", obj.object_type),
                    body: format!("Created at epoch {}", obj.created_epoch),
                    space,
                    timestamp_epoch: obj.created_epoch,
                    action_route: format!("nex://object/{:?}", obj.object_type),
                });
            }
        }

        items.sort_by(|a, b| b.timestamp_epoch.cmp(&a.timestamp_epoch));
        items
    }
}
