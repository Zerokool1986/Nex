use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;

use nex_core::runtime::node::NexNode;
use nex_core::runtime::production::NodeOperationalState;

use crate::ui::NexUiState;

pub struct NexDesktopApp {
    pub node: NexNode,
    pub data_dir: PathBuf,
    pub ui: NexUiState,
    pub status: AppStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Running,
    Error(String),
}

impl NexDesktopApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let data_dir = PathBuf::from("d:\\Nex\\nex_desktop_data");

        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);

        let mut node = NexNode::new(&data_dir, signing_key);
        let status = match node.start() {
            Ok(_) => AppStatus::Running,
            Err(e) => AppStatus::Error(e),
        };

        Self { node, data_dir, ui: NexUiState::new(), status }
    }

    pub fn actor_id_short(&self) -> String {
        hex::encode(&self.node.identity.actor_id[0..4])
    }

    pub fn sync_status(&self) -> &'static str {
        match self.node.operational_state {
            NodeOperationalState::Running => "● Online",
            NodeOperationalState::Degraded => "⚠ Degraded",
            _ => "○ Starting",
        }
    }

    pub fn object_count(&self) -> usize {
        self.node.state.object_store.values().filter(|o| !o.tombstoned).count()
    }
}

impl eframe::App for NexDesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::ui::render(ctx, self);
    }
}
