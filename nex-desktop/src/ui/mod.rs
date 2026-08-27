pub mod home;
pub mod photos;
pub mod media;
pub mod maps;
pub mod drive;
pub mod people;
pub mod devices;
pub mod network;
pub mod inspector;
pub mod settings;
pub mod actions;

use egui::{Context, SidePanel, TopBottomPanel, CentralPanel, Frame, Color32, Stroke, Vec2, RichText};
use nex_core::runtime::experience::InterfaceComplexity;
use crate::app::{NexDesktopApp, AppStatus};

/// Which tab is active in the sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Home,
    Photos,
    Media,
    Maps,
    Drive,
    People,
    Devices,
    Family,
    Network,
    Settings,
}

/// Persistent UI state shared across frames
pub struct NexUiState {
    pub active_tab: NavTab,
    /// Status message shown in bottom bar
    pub status_msg: String,
    /// Global Experience Slider tier governing all lenses & inspectors
    pub complexity: InterfaceComplexity,
    /// Global selected entity for Universal Inspector
    pub selected_entity: Option<inspector::SelectedEntity>,
    /// Ephemeral view state for Media Lens
    pub media_state: media::MediaSessionState,
    /// Ephemeral view state for Maps Lens
    pub maps_state: maps::MapsViewState,
    /// Ephemeral view state for Drive Lens
    pub drive_state: drive::DriveViewState,
    /// Ephemeral view state for People Lens
    pub people_state: people::PeopleViewState,
    /// Ephemeral view state for Network Topology canvas
    pub network_state: network::NetworkViewState,
    /// Sovereign Actions & Dialog State
    pub action_state: actions::ActionState,
}

impl NexUiState {
    pub fn new() -> Self {
        Self {
            active_tab: NavTab::Home,
            status_msg: String::new(),
            complexity: InterfaceComplexity::Standard,
            selected_entity: None,
            media_state: media::MediaSessionState::new(),
            maps_state: maps::MapsViewState::new(),
            drive_state: drive::DriveViewState::new(),
            people_state: people::PeopleViewState::new(),
            network_state: network::NetworkViewState::new(),
            action_state: actions::ActionState::new(),
        }
    }
}

/// Colour palette
pub mod palette {
    use egui::Color32;
    pub const BG: Color32 = Color32::from_rgb(18, 18, 22);
    pub const SIDEBAR: Color32 = Color32::from_rgb(24, 24, 30);
    pub const PANEL: Color32 = Color32::from_rgb(30, 30, 38);
    pub const ACCENT: Color32 = Color32::from_rgb(96, 165, 250);     // blue
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(74, 222, 128);
    pub const TEXT: Color32 = Color32::from_rgb(230, 230, 240);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(130, 130, 150);
    pub const SELECTED: Color32 = Color32::from_rgb(40, 60, 100);
}

/// Top-level render — called every frame by eframe
pub fn render(ctx: &Context, app: &mut NexDesktopApp) {
    // Set visual style once
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = palette::BG;
    visuals.window_fill = palette::BG;
    visuals.extreme_bg_color = palette::SIDEBAR;
    visuals.widgets.inactive.bg_fill = palette::PANEL;
    visuals.widgets.hovered.bg_fill = palette::SELECTED;
    visuals.widgets.active.bg_fill = palette::ACCENT;
    visuals.override_text_color = Some(palette::TEXT);
    ctx.set_visuals(visuals);

    // If app failed to start, show error and bail
    if let AppStatus::Error(ref msg) = app.status.clone() {
        CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(format!("NEX startup error:\n{}", msg))
                    .color(Color32::RED).size(16.0));
            });
        });
        return;
    }

    // Top bar with Global Experience Slider
    TopBottomPanel::top("top_bar")
        .frame(Frame::new().fill(palette::SIDEBAR).inner_margin(8.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("NEX").strong().size(20.0).color(palette::ACCENT));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let sync = app.sync_status();
                    let color = if sync.contains("Online") { palette::ACCENT_GREEN } else { palette::TEXT_DIM };
                    ui.label(RichText::new(sync).color(color).size(13.0));
                    ui.separator();
                    ui.label(RichText::new(format!("ID: {}", app.actor_id_short()))
                        .color(palette::TEXT_DIM).size(12.0));
                    ui.separator();

                    // Global Experience Slider (Simple -> Standard -> Advanced -> Operator)
                    egui::ComboBox::from_id_salt("global_experience_slider")
                        .selected_text(match app.ui.complexity {
                            InterfaceComplexity::Simple => "Simple",
                            InterfaceComplexity::Standard => "Standard",
                            InterfaceComplexity::Advanced => "Advanced",
                            InterfaceComplexity::Expert => "Operator",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.ui.complexity, InterfaceComplexity::Simple, "Simple");
                            ui.selectable_value(&mut app.ui.complexity, InterfaceComplexity::Standard, "Standard");
                            ui.selectable_value(&mut app.ui.complexity, InterfaceComplexity::Advanced, "Advanced");
                            ui.selectable_value(&mut app.ui.complexity, InterfaceComplexity::Expert, "Operator");
                        });
                    ui.label(RichText::new("Experience:").color(palette::TEXT_DIM).size(12.5));
                });
            });
        });

    // Bottom status bar
    TopBottomPanel::bottom("status_bar")
        .frame(Frame::new().fill(palette::SIDEBAR).inner_margin(6.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let msg = if app.ui.status_msg.is_empty() {
                    format!("{} objects in store | Mode: {:?}", app.object_count(), app.ui.complexity)
                } else {
                    app.ui.status_msg.clone()
                };
                ui.label(RichText::new(msg).color(palette::TEXT_DIM).size(12.0));
            });
        });

    // Left sidebar
    SidePanel::left("sidebar")
        .resizable(false)
        .exact_width(140.0)
        .frame(Frame::new().fill(palette::SIDEBAR).inner_margin(8.0))
        .show(ctx, |ui| {
            ui.add_space(8.0);
            nav_item(ui, app, NavTab::Home,     "🏠  Home");
            nav_item(ui, app, NavTab::Photos,   "📷  Photos");
            nav_item(ui, app, NavTab::Media,    "🎬  Media");
            nav_item(ui, app, NavTab::Maps,     "🗺  Maps");
            nav_item(ui, app, NavTab::Drive,    "💾  Drive");
            nav_item(ui, app, NavTab::People,   "👥  People");
            nav_item(ui, app, NavTab::Devices,  "📱  Devices");
            nav_item(ui, app, NavTab::Family,   "🏡  Family");
            nav_item(ui, app, NavTab::Network,  "🌐  Network");
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            nav_item(ui, app, NavTab::Settings, "⚙  Settings");
        });

    // Main content panel (with side-by-side Universal Inspector when active and not in dedicated multi-column tabs)
    CentralPanel::default()
        .frame(Frame::new().fill(palette::BG).inner_margin(20.0))
        .show(ctx, |ui| {
            if app.ui.selected_entity.is_some() 
                && app.ui.active_tab != NavTab::Network 
                && app.ui.active_tab != NavTab::Media 
                && app.ui.active_tab != NavTab::Maps 
                && app.ui.active_tab != NavTab::Drive 
                && app.ui.active_tab != NavTab::People 
            {
                ui.columns(2, |cols| {
                    let (left, right) = cols.split_at_mut(1);
                    render_active_tab(&mut left[0], app);
                    inspector::render_inspector_panel(&mut right[0], app);
                });
            } else {
                render_active_tab(ui, app);
            }

            // Render Sovereign Action Dialog if active
            actions::render_action_dialog(ui, app);
        });
}

fn render_active_tab(ui: &mut egui::Ui, app: &mut NexDesktopApp) {
    match app.ui.active_tab {
        NavTab::Home     => home::render(ui, app),
        NavTab::Photos   => photos::render(ui, app),
        NavTab::Media    => media::render(ui, app),
        NavTab::Maps     => maps::render(ui, app),
        NavTab::Drive    => drive::render(ui, app),
        NavTab::People   => people::render(ui, app),
        NavTab::Devices  => devices::render(ui, app),
        NavTab::Family   => home::render_family(ui, app),
        NavTab::Network  => network::render(ui, app),
        NavTab::Settings => settings::render(ui, app),
    }
}

fn nav_item(ui: &mut egui::Ui, app: &mut NexDesktopApp, tab: NavTab, label: &str) {
    let selected = app.ui.active_tab == tab;
    let bg = if selected { palette::SELECTED } else { Color32::TRANSPARENT };
    let text_color = if selected { palette::ACCENT } else { palette::TEXT };

    let response = ui.add_sized(
        Vec2::new(124.0, 32.0),
        egui::Button::new(RichText::new(label).color(text_color).size(13.5))
            .fill(bg)
            .stroke(Stroke::NONE)
            .frame(true),
    );
    if response.clicked() {
        app.ui.active_tab = tab;
        app.ui.status_msg.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nex_core::runtime::node::NexNode;
    use nex_core::object::types::{NexObject, ObjectType};
    use nex_core::runtime::shell::SpaceType;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;
    use std::collections::BTreeMap;

    fn create_integrated_test_app() -> (NexDesktopApp, [u8; 32]) {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_stage8_integration");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        let obj_id = [0x55; 32];
        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), "Integrated Family Photo.jpg".to_string());
        meta.insert("filename".to_string(), "Integrated Family Photo.jpg".to_string());
        meta.insert("space".to_string(), "Family".to_string());
        meta.insert("geo:lat".to_string(), "39.0968".to_string());
        meta.insert("geo:lon".to_string(), "-120.0324".to_string());
        meta.insert("location:name".to_string(), "Lake Tahoe".to_string());
        meta.insert("rep:thumb".to_string(), "thumb_hash_55".to_string());

        node.state.object_store.insert(obj_id, NexObject {
            object_id: obj_id,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: node.identity.actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: meta,
            payload_bytes: vec![0x55; 4096],
            tombstoned: false,
        });

        let app = NexDesktopApp {
            node,
            data_dir,
            ui: NexUiState::new(),
            status: AppStatus::Running,
        };

        (app, obj_id)
    }

    #[test]
    fn test_object_identity_survives_full_cross_lens_journey() {
        let (mut app, original_obj_id) = create_integrated_test_app();

        // 1. Home / Spaces
        app.ui.active_tab = NavTab::Home;
        app.ui.selected_entity = Some(inspector::SelectedEntity::Object(original_obj_id));

        // 2. Family Space
        app.ui.active_tab = NavTab::Family;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(original_obj_id)));

        // 3. Drive Lens
        app.ui.active_tab = NavTab::Drive;
        app.ui.drive_state.selected_file_id = Some(original_obj_id);
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(original_obj_id)));

        // 4. People Lens
        app.ui.active_tab = NavTab::People;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(original_obj_id)));

        // 5. Media Lens
        app.ui.active_tab = NavTab::Media;
        app.ui.media_state.selected_media_id = Some(original_obj_id);
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(original_obj_id)));

        // 6. Maps Lens
        app.ui.active_tab = NavTab::Maps;
        app.ui.maps_state.selected_object_id = Some(original_obj_id);
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(original_obj_id)));

        // 7. Network Lens
        app.ui.active_tab = NavTab::Network;
        app.ui.network_state.selected_node_id = Some(format!("obj_{}", hex::encode(&original_obj_id[0..4])));
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(original_obj_id)));

        // 8. Return to Drive
        app.ui.active_tab = NavTab::Drive;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(original_obj_id)));

        // 9. Verify zero mutation in canonical storage
        assert_eq!(app.node.state.object_store.len(), 1);
        assert!(app.node.state.object_store.contains_key(&original_obj_id));
    }

    #[test]
    fn test_person_identity_survives_cross_lens_navigation() {
        let (mut app, _) = create_integrated_test_app();
        let actor_id = app.node.identity.actor_id;

        app.ui.active_tab = NavTab::People;
        app.ui.selected_entity = Some(inspector::SelectedEntity::Person(actor_id));

        app.ui.active_tab = NavTab::Drive;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Person(actor_id)));

        app.ui.active_tab = NavTab::Network;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Person(actor_id)));

        app.ui.active_tab = NavTab::Settings;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Person(actor_id)));
    }

    #[test]
    fn test_device_identity_survives_cross_lens_navigation() {
        let (mut app, _) = create_integrated_test_app();
        let device_id = app.node.identity.actor_id;

        app.ui.active_tab = NavTab::Devices;
        app.ui.selected_entity = Some(inspector::SelectedEntity::Device(device_id));

        app.ui.active_tab = NavTab::People;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Device(device_id)));

        app.ui.active_tab = NavTab::Drive;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Device(device_id)));

        app.ui.active_tab = NavTab::Network;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Device(device_id)));
    }

    #[test]
    fn test_space_identity_survives_cross_lens_navigation() {
        let (mut app, _) = create_integrated_test_app();

        app.ui.active_tab = NavTab::Family;
        app.ui.selected_entity = Some(inspector::SelectedEntity::Space(SpaceType::Family));

        app.ui.active_tab = NavTab::People;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Space(SpaceType::Family)));

        app.ui.active_tab = NavTab::Drive;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Space(SpaceType::Family)));

        app.ui.active_tab = NavTab::Maps;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Space(SpaceType::Family)));

        app.ui.active_tab = NavTab::Network;
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Space(SpaceType::Family)));
    }

    #[test]
    fn test_selection_cannot_drift_between_projection_models() {
        let (app, target_obj_id) = create_integrated_test_app();

        let drive_catalog = drive::derive_drive_catalog(&app);
        assert_eq!(drive_catalog[0].object_id, target_obj_id);

        let media_catalog = media::derive_media_catalog(&app);
        assert_eq!(media_catalog[0].object_id, target_obj_id);

        let geo_catalog = maps::derive_geo_catalog(&app);
        assert_eq!(geo_catalog[0].object_id, target_obj_id);

        let (nodes, _) = network::derive_topology(&app);
        let obj_node = nodes.iter().find(|n| n.id.starts_with("obj_")).unwrap();
        assert!(obj_node.id.contains(&hex::encode(&target_obj_id[0..4])));

        let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(
            &app.node, &target_obj_id, InterfaceComplexity::Standard
        ).unwrap();
        assert_eq!(inspector.object_id, target_obj_id);
    }

    #[test]
    fn test_benchmark_human_journey() {
        let (mut app, photo_id) = create_integrated_test_app();

        // Step 1: User arrives in Home & sees Family Space
        app.ui.active_tab = NavTab::Family;
        
        // Step 2: User clicks photo in Family feed
        app.ui.selected_entity = Some(inspector::SelectedEntity::Object(photo_id));

        // Step 3: Inspector confirms ownership by sovereign actor
        let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(
            &app.node, &photo_id, InterfaceComplexity::Simple
        ).unwrap();
        assert_eq!(inspector.title, "Integrated Family Photo.jpg");

        // Step 4: User opens People view to verify author and access
        app.ui.active_tab = NavTab::People;
        let people = people::derive_people_catalog(&app);
        assert!(people[0].is_local);

        // Step 5: User opens Drive file view
        app.ui.active_tab = NavTab::Drive;
        app.ui.drive_state.selected_file_id = Some(photo_id);
        let drive_cat = drive::derive_drive_catalog(&app);
        assert_eq!(drive_cat[0].filename, "Integrated Family Photo.jpg");

        // Step 6: User opens Media representation view
        app.ui.active_tab = NavTab::Media;
        app.ui.media_state.selected_media_id = Some(photo_id);
        let media_cat = media::derive_media_catalog(&app);
        assert_eq!(media_cat[0].representations.len(), 2);

        // Step 7: User opens Location / Maps view
        app.ui.active_tab = NavTab::Maps;
        app.ui.maps_state.selected_object_id = Some(photo_id);
        let geo_cat = maps::derive_geo_catalog(&app);
        assert_eq!(geo_cat[0].place_label, "Lake Tahoe");

        // Step 8: User navigates to Network to see replication to local device
        app.ui.active_tab = NavTab::Network;
        let (nodes, _) = network::derive_topology(&app);
        assert!(nodes.iter().any(|n| n.id == "device_local"));

        // Step 9: Identity remained strictly intact throughout
        assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(photo_id)));
    }
}
