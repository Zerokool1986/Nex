use std::collections::BTreeMap;
use crate::runtime::node::NexNode;
use crate::runtime::shell::{NexHomeShell, SpaceType};
use crate::runtime::experience::{HumanExperienceEngine, HomeScreenViewModel, InterfaceComplexity};
use crate::object::types::ObjectID;

pub struct NexHomeController;

impl NexHomeController {
    pub fn open_home(
        node: &NexNode,
        active_space: SpaceType,
        complexity: InterfaceComplexity,
    ) -> HomeScreenViewModel {
        HumanExperienceEngine::render_home_screen(node, active_space, complexity)
    }

    pub fn list_available_spaces() -> Vec<SpaceType> {
        vec![
            SpaceType::Personal,
            SpaceType::Family,
            SpaceType::Work,
            SpaceType::Community,
            SpaceType::Project,
        ]
    }
}
