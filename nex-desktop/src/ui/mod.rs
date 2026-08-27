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
pub mod palette_command;
pub mod chat;

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
    Chat,
    Inspector,
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
    /// Ephemeral view state for Devices Lens
    pub devices_state: devices::DevicesViewState,
    /// Ephemeral view state for Network Topology canvas
    pub network_state: network::NetworkViewState,
    /// Ephemeral view state for Comms Chat
    pub chat_state: chat::ChatViewState,
    /// Sovereign Actions & Dialog State
    pub action_state: actions::ActionState,
    /// Global Command Palette / Spotlight Launcher state
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub command_palette_state: palette_command::CommandPaletteState,
    /// Keyboard traversal index for Home feed
    pub home_selected_index: Option<usize>,
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
            devices_state: devices::DevicesViewState::new(),
            network_state: network::NetworkViewState::new(),
            chat_state: chat::ChatViewState::new(),
            action_state: actions::ActionState::new(),
            command_palette_open: false,
            command_palette_query: String::new(),
            command_palette_state: palette_command::CommandPaletteState::new(),
            home_selected_index: None,
        }
    }
}

/// Colour palette — Calm Sovereignty / Native Precision Hybrid
pub mod palette {
    use egui::Color32;
    // ── Foundation surfaces ──
    pub const BG: Color32 = Color32::from_rgb(14, 15, 20);              // Obsidian canvas background (#0E0F14)
    pub const SIDEBAR: Color32 = Color32::from_rgb(20, 22, 30);         // Distinct sidebar surface (#14161E)
    pub const PANEL: Color32 = Color32::from_rgb(24, 26, 36);           // Elevated container (#181A24)
    pub const PANEL_HOVER: Color32 = Color32::from_rgb(30, 33, 46);     // Hover lift (#1E212E)
    pub const CARD: Color32 = Color32::from_rgb(22, 24, 33);            // Obsidian Glass card (#161821)
    pub const CARD_HOVER: Color32 = Color32::from_rgb(28, 31, 44);      // Elevated card hover (#1C1F2C)
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(36, 40, 56);   // Crisp divider stroke (#242838)
    pub const GLASS_BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 18); // Translucent rim

    // ── Brand & Accent ──
    pub const ACCENT: Color32 = Color32::from_rgb(99, 144, 250);        // Radiant cobalt (#6390FA)
    pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(99, 144, 250, 40); // Ambient glow
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(52, 211, 153);  // Emerald trust (#34D399)
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);  // Amber local (#FBBF24)

    // ── Typography ──
    pub const TEXT: Color32 = Color32::from_rgb(245, 245, 250);         // Primary text (#F5F5FA)
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 163, 182); // Secondary text (#A0A3B6)
    pub const TEXT_DIM: Color32 = Color32::from_rgb(105, 108, 128);     // Tertiary/muted (#696C80)

    // ── Interactive states ──
    pub const SELECTED: Color32 = Color32::from_rgb(28, 36, 58);        // Selection glow (#1C243A)
    pub const NAV_ACTIVE: Color32 = Color32::from_rgb(99, 144, 250);    // Active nav text = accent
    pub const NAV_INACTIVE: Color32 = Color32::from_rgb(145, 148, 166); // Inactive nav (#9194A6)
}

/// Top-level render — called every frame by eframe
pub fn render(ctx: &Context, app: &mut NexDesktopApp) {
    // Set refined visual style
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = palette::BG;
    visuals.window_fill = palette::BG;
    visuals.extreme_bg_color = palette::SIDEBAR;
    visuals.faint_bg_color = palette::PANEL;
    visuals.code_bg_color = palette::CARD;
    visuals.widgets.noninteractive.bg_fill = palette::BG;
    visuals.widgets.inactive.bg_fill = palette::PANEL;
    visuals.widgets.hovered.bg_fill = palette::PANEL_HOVER;
    visuals.widgets.active.bg_fill = palette::ACCENT;
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    visuals.override_text_color = Some(palette::TEXT);

    // Polish spacing & interaction feel
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    ctx.set_style(style);
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

    // Global keyboard shortcuts
    ctx.input(|i| {
        if i.modifiers.command && i.key_pressed(egui::Key::K) {
            app.ui.command_palette_open = !app.ui.command_palette_open;
            if app.ui.command_palette_open {
                app.ui.command_palette_state.reset();
            }
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num1) {
            app.ui.active_tab = NavTab::Home;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num2) {
            app.ui.active_tab = NavTab::Family;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num3) {
            app.ui.active_tab = NavTab::Photos;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num4) {
            app.ui.active_tab = NavTab::Drive;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num5) {
            app.ui.active_tab = NavTab::People;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num6) {
            app.ui.active_tab = NavTab::Devices;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num7) {
            app.ui.active_tab = NavTab::Network;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num8) {
            app.ui.active_tab = NavTab::Maps;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num9) {
            app.ui.active_tab = NavTab::Settings;
            app.ui.status_msg.clear();
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num0) {
            app.ui.active_tab = NavTab::Chat;
            app.ui.status_msg.clear();
        }
    });

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // BOTTOM UTILITY DOCK — Clean keyboard helper & quick status
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    TopBottomPanel::bottom(egui::Id::new("nex_status_bar_v4"))
        .frame(Frame::new()
            .fill(palette::SIDEBAR)
            .inner_margin(egui::Margin { left: 18, right: 18, top: 7, bottom: 7 })
            .stroke(Stroke::new(1.0_f32, palette::BORDER_SUBTLE))
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⌘K Command Menu   •   ⌘1 Personal   •   ⌘2 Family   •   ⌘3 Photos   •   ⌘4 Drive")
                    .color(palette::TEXT_DIM).size(11.5));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !app.ui.status_msg.is_empty() {
                        ui.label(RichText::new(&app.ui.status_msg).color(palette::ACCENT).size(11.5));
                    } else {
                        ui.label(RichText::new("Zero Cloud Dependency • Local Substrate")
                            .color(palette::TEXT_DIM).size(11.0));
                    }
                });
            });
        });

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // UNIFIED MAIN CANVAS — Seamless Sidebar & Workspace Integration
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    CentralPanel::default()
        .frame(Frame::new().fill(palette::BG).inner_margin(egui::Margin::ZERO))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.horizontal(|ui| {
                // ── 1. LEFT NAVIGATION COLUMN (Exact 220px, Obsidian Surface) ──
                Frame::new()
                    .fill(palette::SIDEBAR)
                    .inner_margin(egui::Margin { left: 16, right: 16, top: 20, bottom: 16 })
                    .stroke(Stroke::new(1.0_f32, palette::BORDER_SUBTLE))
                    .show(ui, |ui| {
                        ui.set_width(220.0);
                        ui.set_min_height(ui.available_height());

                        ui.vertical(|ui| {
                            // ── Node Identity Anchor ──
                            ui.horizontal(|ui| {
                                ui.add(egui::Image::new(egui::include_image!("../../assets/nex_brand_icon.png"))
                                    .max_height(26.0)
                                    .max_width(26.0));
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("NEX").strong().size(16.5).color(palette::TEXT));
                                    let sync = app.sync_status();
                                    let (color, dot) = if sync.contains("Online") {
                                        (palette::ACCENT_GREEN, "●")
                                    } else if sync.contains("Degraded") {
                                        (palette::ACCENT_AMBER, "●")
                                    } else {
                                        (palette::TEXT_DIM, "○")
                                    };
                                    ui.label(RichText::new(format!("{} Sovereign Node", dot))
                                        .size(11.0).color(color));
                                });
                            });

                            ui.add_space(16.0);

                            // ── Quick Search Trigger ──
                            let search_response = ui.add_sized(
                                Vec2::new(ui.available_width(), 34.0),
                                egui::Button::new(
                                    RichText::new(format!("{}  Search or Jump… ⌘K", egui_phosphor::regular::MAGNIFYING_GLASS))
                                        .size(12.5).color(palette::TEXT_DIM)
                                )
                                .fill(palette::PANEL)
                                .corner_radius(8.0)
                                .stroke(Stroke::new(1.0_f32, palette::BORDER_SUBTLE)),
                            );
                            if search_response.clicked() {
                                app.ui.command_palette_open = true;
                                app.ui.command_palette_query.clear();
                            }

                            ui.add_space(18.0);

                            // ── SPACES (Trust Boundaries) ──
                            section_header(ui, "SPACES");
                            nav_item(ui, app, NavTab::Home,     egui_phosphor::regular::HOUSE_SIMPLE, "Personal", "⌘1");
                            nav_item(ui, app, NavTab::Family,   egui_phosphor::regular::USERS_THREE, "Family", "⌘2");

                            ui.add_space(16.0);

                            // ── COMMS (Sovereign Channels) ──
                            section_header(ui, "COMMS");
                            nav_item(ui, app, NavTab::Chat,     egui_phosphor::regular::CHATS_TEARDROP, "Chat", "⌘0");

                            ui.add_space(16.0);

                            // ── LENSES (Projections of DAG) ──
                            section_header(ui, "LENSES");
                            nav_item(ui, app, NavTab::Photos,   egui_phosphor::regular::IMAGE, "Photos", "⌘3");
                            nav_item(ui, app, NavTab::Drive,    egui_phosphor::regular::FOLDER_SIMPLE, "Drive", "⌘4");
                            nav_item(ui, app, NavTab::Maps,     egui_phosphor::regular::MAP_TRIFOLD, "Maps", "");

                            ui.add_space(16.0);

                            // ── MESH & TRUST (Hardware & Identity) ──
                            section_header(ui, "MESH & TRUST");
                            nav_item(ui, app, NavTab::People,   egui_phosphor::regular::IDENTIFICATION_CARD, "People", "");
                            nav_item(ui, app, NavTab::Devices,  egui_phosphor::regular::DESKTOP_TOWER, "Devices", "");
                            nav_item(ui, app, NavTab::Network,  egui_phosphor::regular::GRAPH, "Topology", "");

                            // ── BOTTOM DOCK: SETTINGS & COMPLEXITY ──
                            ui.add_space(18.0);
                            ui.add(egui::Separator::default().spacing(1.0));
                            ui.add_space(10.0);

                            // Settings link
                            nav_item(ui, app, NavTab::Settings, egui_phosphor::regular::GEAR_SIX, "Node Settings", "");
                            ui.add_space(10.0);

                            // Progressive Disclosure Experience Slider
                            ui.horizontal(|ui| {
                                let tiers = [
                                    (InterfaceComplexity::Simple, "Simple"),
                                    (InterfaceComplexity::Standard, "Standard"),
                                    (InterfaceComplexity::Advanced, "Adv"),
                                    (InterfaceComplexity::Expert, "Op"),
                                ];
                                for (tier, label) in tiers {
                                    let is_active = app.ui.complexity == tier;
                                    let text_color = if is_active { palette::TEXT } else { palette::TEXT_DIM };
                                    let bg = if is_active { palette::SELECTED } else { Color32::TRANSPARENT };
                                    let stroke = if is_active { Stroke::new(1.0_f32, palette::ACCENT) } else { Stroke::NONE };
                                    if ui.add(
                                        egui::Button::new(RichText::new(label).size(10.5).color(text_color))
                                            .fill(bg)
                                            .stroke(stroke)
                                            .corner_radius(4.0)
                                            .min_size(Vec2::new(38.0, 24.0))
                                    ).clicked() {
                                        app.ui.complexity = tier;
                                    }
                                }
                            });
                        });
                    });

                // ── 2. ACTIVE TAB WORKSPACE (Paints directly adjacent to sidebar) ──
                Frame::new()
                    .fill(palette::BG)
                    .inner_margin(egui::Margin { left: 24, right: 24, top: 24, bottom: 20 })
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(ui.available_height());

                        ui.vertical(|ui| {
                            match app.ui.active_tab {
                                NavTab::Home      => home::render(ui, app),
                                NavTab::Photos    => photos::render(ui, app),
                                NavTab::Media     => media::render(ui, app),
                                NavTab::Maps      => maps::render(ui, app),
                                NavTab::Drive     => drive::render(ui, app),
                                NavTab::People    => people::render(ui, app),
                                NavTab::Devices   => devices::render(ui, app),
                                NavTab::Family    => home::render_family(ui, app),
                                NavTab::Network   => network::render(ui, app),
                                NavTab::Settings  => settings::render(ui, app),
                                NavTab::Chat      => chat::render(ui, app),
                                NavTab::Inspector => inspector::render(ui, app),
                            }

                            // Trigger action modal dialogs (Import, Export, Proximity SAS Verification)
                            actions::render_action_dialog(ui, app);
                        });
                    });
            });
        });

    // Global Command Palette / Spotlight Launcher Modal
    let mut palette_state = app.ui.command_palette_state.clone();
    palette_command::render_command_palette(ctx, app, &mut palette_state);
    app.ui.command_palette_state = palette_state;
}

fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label)
        .size(10.5)
        .color(palette::TEXT_DIM)
        .strong()
    );
    ui.add_space(3.0);
}

fn nav_item(ui: &mut egui::Ui, app: &mut NexDesktopApp, tab: NavTab, icon: &str, label: &str, shortcut: &str) {
    let selected = app.ui.active_tab == tab;
    let bg = if selected { palette::SELECTED } else { Color32::TRANSPARENT };
    let text_color = if selected { palette::NAV_ACTIVE } else { palette::NAV_INACTIVE };
    let stroke = if selected { Stroke::new(1.0_f32, palette::ACCENT) } else { Stroke::NONE };

    let response = ui.add_sized(
        Vec2::new(ui.available_width(), 32.0),
        egui::Button::new(
            RichText::new(format!("{}   {}", icon, label))
                .color(text_color)
                .size(13.2)
        )
        .fill(bg)
        .stroke(stroke)
        .corner_radius(6.0)
        .frame(true),
    );

    if !shortcut.is_empty() {
        let rect = response.rect;
        ui.painter().text(
            egui::Pos2::new(rect.right() - 8.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            shortcut,
            egui::FontId::proportional(10.5),
            palette::TEXT_DIM,
        );
    }

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

        let app = NexDesktopApp::new_test(node, data_dir);

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

    #[test]
    fn test_exhaustive_ui_render_all_tabs_and_complexity_levels() {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        ctx.set_fonts(fonts);

        let (mut app, _photo_id) = create_integrated_test_app();

        let tabs = [
            NavTab::Home,
            NavTab::Family,
            NavTab::Photos,
            NavTab::Drive,
            NavTab::Media,
            NavTab::Maps,
            NavTab::People,
            NavTab::Devices,
            NavTab::Network,
            NavTab::Settings,
        ];

        let tiers = [
            InterfaceComplexity::Simple,
            InterfaceComplexity::Standard,
            InterfaceComplexity::Advanced,
            InterfaceComplexity::Expert,
        ];

        for tab in tabs {
            for tier in tiers {
                app.ui.active_tab = tab;
                app.ui.complexity = tier;

                let mut raw_input = egui::RawInput::default();
                raw_input.screen_rect = Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1080.0, 700.0),
                ));

                let full_output = ctx.run(raw_input, |ctx| {
                    render(ctx, &mut app);
                });

                assert!(
                    !full_output.shapes.is_empty(),
                    "Tab {:?} under Complexity {:?} must generate valid render shapes",
                    tab,
                    tier
                );
            }
        }
    }
}
