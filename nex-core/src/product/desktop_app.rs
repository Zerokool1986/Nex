use std::path::Path;
use crate::runtime::node::NexNode;
use crate::runtime::shell::SpaceType;
use crate::runtime::experience::InterfaceComplexity;
use crate::product::home::NexHomeController;
use crate::product::inspector::UniversalObjectInspector;
use crate::product::person::{PersonPanelController, TrustTier};
use crate::product::device::DevicePanelController;
use crate::product::settings::SettingsController;
use crate::product::ingest::LocalFileIngestor;
use crate::object::types::ObjectID;
use crate::identity::types::{ActorID, CapabilityProof};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopNavigationTab {
    Home,
    Photos,
    Drive,
    People,
    Devices,
    Settings,
}

#[derive(Debug, Clone)]
pub struct DesktopAppSession {
    pub active_tab: DesktopNavigationTab,
    pub active_space: SpaceType,
    pub complexity: InterfaceComplexity,
    pub selected_object_id: Option<ObjectID>,
    pub selected_person: Option<(ActorID, String)>,
    pub selected_device: Option<(ActorID, String)>,
    pub is_hardware_keystore_verified: bool,
    pub status_message: String,
}

impl DesktopAppSession {
    pub fn new() -> Self {
        Self {
            active_tab: DesktopNavigationTab::Home,
            active_space: SpaceType::Family,
            complexity: InterfaceComplexity::Standard,
            selected_object_id: None,
            selected_person: None,
            selected_device: None,
            is_hardware_keystore_verified: false, // Default truthful state on desktop host
            status_message: "NEX Desktop Ready".to_string(),
        }
    }

    pub fn select_tab(&mut self, tab: DesktopNavigationTab) {
        self.active_tab = tab;
    }

    pub fn select_space(&mut self, space: SpaceType) {
        self.active_space = space;
        self.selected_object_id = None;
        self.status_message = format!("Switched to {:?} Space", space);
    }

    pub fn set_complexity_slider(&mut self, level: InterfaceComplexity) {
        self.complexity = level;
        self.status_message = format!("Interface Complexity: {:?}", level);
    }

    pub fn inspect_object(&mut self, object_id: ObjectID) {
        self.selected_object_id = Some(object_id);
    }

    pub fn open_person(&mut self, actor_id: ActorID, name: &str) {
        self.active_tab = DesktopNavigationTab::People;
        self.selected_person = Some((actor_id, name.to_string()));
    }

    pub fn open_device(&mut self, actor_id: ActorID, name: &str) {
        self.active_tab = DesktopNavigationTab::Devices;
        self.selected_device = Some((actor_id, name.to_string()));
    }

    pub fn import_local_file(
        &mut self,
        node: &mut NexNode,
        file_path: &Path,
        proof: &CapabilityProof,
        actor_id: &ActorID,
        current_epoch: u64,
    ) -> Result<ObjectID, String> {
        let object_id = LocalFileIngestor::ingest_file(
            node,
            self.active_space,
            file_path,
            proof,
            actor_id,
            current_epoch,
        )?;
        self.selected_object_id = Some(object_id);
        self.status_message = format!("Imported '{}' into {:?} Space", file_path.display(), self.active_space);
        Ok(object_id)
    }

    /// Renders the complete live visual screen buffer for the active desktop view
    pub fn render_view_string(&self, node: &NexNode) -> String {
        let mut out = String::new();
        out.push_str("================================================================================\n");
        out.push_str(&format!("  NEX DESKTOP  |  Space: {:?}  |  Level: {:?}  |  Status: {}\n", self.active_space, self.complexity, self.status_message));
        out.push_str("================================================================================\n");
        out.push_str("  [1] Home  |  [2] Photos  |  [3] Drive  |  [4] People  |  [5] Devices  |  [6] Settings\n");
        out.push_str("--------------------------------------------------------------------------------\n\n");

        match self.active_tab {
            DesktopNavigationTab::Home => {
                let home_vm = NexHomeController::open_home(node, self.active_space, self.complexity);
                out.push_str(&format!("🏠 HOME — {}\n", home_vm.space_title));
                out.push_str(&format!("   Sync Status : {}\n", home_vm.sync_status_label));
                out.push_str(&format!("   Protection  : {}\n", home_vm.identity_protection_label));
                out.push_str(&format!("   Total Items : {}\n\n", home_vm.total_items_in_space));
                out.push_str("   Recent Feed:\n");
                if home_vm.feed_items.is_empty() {
                    out.push_str("   (No items in this Space yet. Import photos or files to begin.)\n");
                } else {
                    for (i, item) in home_vm.feed_items.iter().enumerate() {
                        out.push_str(&format!("   [{}] {} ({})\n", i + 1, item.title, item.status_badge));
                    }
                }
            }
            DesktopNavigationTab::Photos => {
                let photos_vm = crate::runtime::experience::HumanExperienceEngine::render_photos_screen(node, self.active_space, self.complexity);
                out.push_str(&format!("📷 PHOTOS LENS — {:?} Space\n", photos_vm.active_space));
                out.push_str(&format!("   Total Photos : {}\n", photos_vm.total_photos));
                out.push_str(&format!("   Storage Used : {}\n\n", photos_vm.storage_used_label));
                if photos_vm.photo_cards.is_empty() {
                    out.push_str("   (No photos in this Space. Use file import to add photos.)\n");
                } else {
                    for card in &photos_vm.photo_cards {
                        out.push_str(&format!("   • {} [{}] — {}\n", card.title, card.byte_size_formatted, card.status_badge));
                    }
                }
            }
            DesktopNavigationTab::Drive => {
                let drive_vm = crate::runtime::experience::HumanExperienceEngine::render_drive_screen(node, self.active_space, self.complexity);
                out.push_str(&format!("📁 DRIVE LENS — {:?} Space\n", drive_vm.active_space));
                out.push_str(&format!("   Total Files  : {}\n", drive_vm.total_files));
                out.push_str(&format!("   Storage Used : {}\n\n", drive_vm.storage_used_label));
                if drive_vm.file_rows.is_empty() {
                    out.push_str("   (No documents in this Space. Use file import to add files.)\n");
                } else {
                    for item in &drive_vm.file_rows {
                        out.push_str(&format!("   • {} [{}] — {}\n", item.filename, item.byte_size_formatted, item.status_badge));
                    }
                }
            }
            DesktopNavigationTab::People => {
                let amy_actor = [0xAA; 32];
                let (target_actor, name) = self.selected_person.as_ref()
                    .map(|(a, n)| (*a, n.as_str()))
                    .unwrap_or((amy_actor, "Amy"));

                let person_vm = PersonPanelController::build_person_surface(
                    node,
                    &target_actor,
                    name,
                    TrustTier::VerifiedSovereignPeer,
                    self.complexity,
                );

                out.push_str(&format!("👤 PERSON PANEL — {}\n", person_vm.display_name));
                out.push_str(&format!("   Trust State : {}\n", person_vm.trust_badge));
                out.push_str(&format!("   Connection  : {}\n", person_vm.connection_type_label));
                out.push_str(&format!("   Shared Items: {} objects\n", person_vm.shared_objects_count));
                out.push_str("   Quick Actions:\n");
                for action in &person_vm.quick_actions {
                    out.push_str(&format!("     [ {} ]", action));
                }
                out.push_str("\n");
                if let Some(tech) = person_vm.technical_identity_info {
                    out.push_str(&format!("\n   🔬 Advanced Identity: {}\n", tech));
                }
            }
            DesktopNavigationTab::Devices => {
                let (target_actor, name) = self.selected_device.as_ref()
                    .map(|(a, n)| (*a, n.as_str()))
                    .unwrap_or((node.identity.actor_id, "Chris's Desktop Station"));

                let dev_vm = DevicePanelController::build_device_surface(
                    node,
                    &target_actor,
                    name,
                    None,
                    false,
                    self.is_hardware_keystore_verified,
                    self.complexity,
                );

                out.push_str(&format!("📱 DEVICE PANEL — {}\n", dev_vm.device_name));
                out.push_str(&format!("   Status      : {}\n", dev_vm.connection_badge));
                out.push_str(&format!("   Transport   : {}\n", dev_vm.transport_type_label));
                out.push_str(&format!("   Latency     : {} ms\n", dev_vm.latency_ms));
                out.push_str(&format!("   Storage     : {}\n", dev_vm.storage_quota_label));
                out.push_str(&format!("   Protection  : {}\n", dev_vm.key_protection_status));
                if let Some(tech) = dev_vm.technical_device_info {
                    out.push_str(&format!("\n   🔬 Diagnostics: {}\n", tech));
                }
            }
            DesktopNavigationTab::Settings => {
                let settings_tree = SettingsController::build_settings_tree(self.complexity);
                out.push_str("⚙️ SETTINGS & EXPERIENCE SLIDER\n");
                out.push_str(&format!("   Active Slider: {:?}\n\n", settings_tree.active_complexity_slider));
                out.push_str("   User & Identity:\n");
                for item in &settings_tree.user_section {
                    out.push_str(&format!("     • {}: {} ({})\n", item.title, item.current_value, item.explanation));
                }
                out.push_str("\n   Your NEX System:\n");
                for item in &settings_tree.your_nex_section {
                    out.push_str(&format!("     • {}: {} ({})\n", item.title, item.current_value, item.explanation));
                }
                if let Some(adv) = settings_tree.advanced_section {
                    out.push_str("\n   🔬 Cryptographic & Storage Diagnostics:\n");
                    for item in adv {
                        out.push_str(&format!("     • {}: {} ({})\n", item.title, item.current_value, item.explanation));
                    }
                }
            }
        }

        // Render Universal Object Inspector Drawer if an object is selected
        if let Some(obj_id) = self.selected_object_id {
            if let Ok(insp) = UniversalObjectInspector::inspect(node, &obj_id, self.complexity) {
                out.push_str("\n--------------------------------------------------------------------------------\n");
                out.push_str(&format!("🔍 UNIVERSAL OBJECT INSPECTOR — {}\n", insp.title));
                out.push_str(&format!("   Type       : {:?}  |  Space: {}  |  Size: {}\n", insp.object_type, insp.space_name, insp.byte_size_formatted));
                out.push_str(&format!("   Status     : {}\n", insp.status_badge));
                out.push_str(&format!("   Replicas   : {} stored on {:?}\n", insp.replica_count, insp.stored_on_devices));
                out.push_str(&format!("   Shared With: {:?}\n", insp.shared_with_peers));
                out.push_str(&format!("   Actions    : {:?}\n", insp.available_capabilities));
                if let Some(dag) = insp.advanced_dag_info {
                    out.push_str(&format!("   🔬 DAG Info: Schema v{}, Created Epoch {}, CAS Chunks: {}\n", dag.schema_version, dag.created_epoch, dag.cas_chunk_count));
                }
            }
        }

        out.push_str("================================================================================\n");
        out
    }
}
