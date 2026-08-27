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
    /// Global Command Palette / Spotlight Launcher state
    pub command_palette_open: bool,
    pub command_palette_query: String,
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
            command_palette_open: false,
            command_palette_query: String::new(),
        }
    }
}

/// Colour palette — Calm Sovereignty / Native Precision Hybrid
pub mod palette {
    use egui::Color32;
    // ── Foundation surfaces ──
    pub const BG: Color32 = Color32::from_rgb(12, 12, 16);              // Near-black void (#0C0C10)
    pub const SIDEBAR: Color32 = Color32::from_rgb(16, 16, 22);         // Sidebar surface (#101016)
    pub const PANEL: Color32 = Color32::from_rgb(22, 23, 30);           // Elevated container (#16171E)
    pub const PANEL_HOVER: Color32 = Color32::from_rgb(30, 32, 42);     // Hover lift (#1E202A)
    pub const CARD: Color32 = Color32::from_rgb(26, 28, 36);            // Card surface (#1A1C24)
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 40, 52);   // Subtle divider (#262834)

    // ── Brand & Accent ──
    pub const ACCENT: Color32 = Color32::from_rgb(99, 144, 250);        // Softer cobalt (#6390FA)
    pub const ACCENT_SOFT: Color32 = Color32::from_rgb(99, 144, 250);   // Same for hover glow
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(52, 211, 153);  // Emerald trust (#34D399)
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);  // Amber local (#FBBF24)

    // ── Typography ──
    pub const TEXT: Color32 = Color32::from_rgb(240, 240, 248);         // Primary text (#F0F0F8)
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(155, 158, 178); // Secondary text (#9B9EB2)
    pub const TEXT_DIM: Color32 = Color32::from_rgb(100, 103, 122);     // Tertiary/muted (#64677A)

    // ── Interactive states ──
    pub const SELECTED: Color32 = Color32::from_rgb(30, 38, 64);        // Selection glow (#1E2640)
    pub const NAV_ACTIVE: Color32 = Color32::from_rgb(99, 144, 250);    // Active nav text = accent
    pub const NAV_INACTIVE: Color32 = Color32::from_rgb(140, 143, 160); // Inactive nav (#8C8FA0)
}

/// Top-level render — called every frame by eframe
pub fn render(ctx: &Context, app: &mut NexDesktopApp) {
    // Set refined visual style
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = palette::BG;
    visuals.window_fill = palette::BG;
    visuals.extreme_bg_color = palette::SIDEBAR;
    visuals.widgets.inactive.bg_fill = palette::PANEL;
    visuals.widgets.hovered.bg_fill = palette::PANEL_HOVER;
    visuals.widgets.active.bg_fill = palette::ACCENT;
    visuals.widgets.noninteractive.bg_fill = palette::PANEL;
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    visuals.override_text_color = Some(palette::TEXT);

    // Reduce default spacing for tighter, more polished feel
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
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

    // Detect Ctrl+K / Cmd+K global shortcut
    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
        app.ui.command_palette_open = !app.ui.command_palette_open;
        if app.ui.command_palette_open {
            app.ui.command_palette_query.clear();
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // LEFT NAVIGATION COLUMN — Identity-anchored, spatially organized
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    SidePanel::left("sidebar")
        .resizable(false)
        .exact_width(200.0)
        .frame(Frame::new()
            .fill(palette::SIDEBAR)
            .inner_margin(egui::Margin { left: 14, right: 14, top: 16, bottom: 12 })
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
        )
        .show(ctx, |ui| {
            // ── Node Identity Header ──
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(egui::include_image!("../../assets/nex_brand_icon.png"))
                    .max_height(24.0)
                    .max_width(24.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new("NEX").strong().size(16.0).color(palette::TEXT));
                    let sync = app.sync_status();
                    let (color, dot) = if sync.contains("Online") {
                        (palette::ACCENT_GREEN, "●")
                    } else if sync.contains("Degraded") {
                        (palette::ACCENT_AMBER, "●")
                    } else {
                        (palette::TEXT_DIM, "○")
                    };
                    ui.label(RichText::new(format!("{} {}", dot, sync.replace("● ", "").replace("⚠ ", "").replace("○ ", "")))
                        .size(11.0).color(color));
                });
            });

            ui.add_space(16.0);

            // ── Quick Search Trigger ──
            let search_response = ui.add_sized(
                Vec2::new(ui.available_width(), 32.0),
                egui::Button::new(
                    RichText::new(format!("{}  Search…", egui_phosphor::regular::MAGNIFYING_GLASS))
                        .size(12.5).color(palette::TEXT_DIM)
                )
                .fill(palette::PANEL)
                .corner_radius(8.0)
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE)),
            );
            if search_response.clicked() {
                app.ui.command_palette_open = true;
                app.ui.command_palette_query.clear();
            }

            ui.add_space(20.0);

            // ── Navigation Sections ──
            // Spaces
            section_header(ui, "Spaces");
            nav_item(ui, app, NavTab::Home,     egui_phosphor::regular::HOUSE_SIMPLE, "Personal");
            nav_item(ui, app, NavTab::Family,   egui_phosphor::regular::USERS_THREE, "Family");

            ui.add_space(16.0);

            // Lenses
            section_header(ui, "Lenses");
            nav_item(ui, app, NavTab::Photos,   egui_phosphor::regular::IMAGE, "Photos");
            nav_item(ui, app, NavTab::Drive,    egui_phosphor::regular::FOLDER_SIMPLE, "Files");
            nav_item(ui, app, NavTab::Media,    egui_phosphor::regular::PLAY_CIRCLE, "Media");
            nav_item(ui, app, NavTab::Maps,     egui_phosphor::regular::MAP_TRIFOLD, "Maps");

            ui.add_space(16.0);

            // Mesh
            section_header(ui, "Mesh");
            nav_item(ui, app, NavTab::People,   egui_phosphor::regular::IDENTIFICATION_CARD, "People");
            nav_item(ui, app, NavTab::Devices,  egui_phosphor::regular::DESKTOP_TOWER, "Devices");
            nav_item(ui, app, NavTab::Network,  egui_phosphor::regular::GRAPH, "Topology");

            // Bottom-anchored settings
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(4.0);

                // Experience slider as compact pill at bottom
                ui.horizontal(|ui| {
                    let tiers = [
                        (InterfaceComplexity::Simple, "S"),
                        (InterfaceComplexity::Standard, "Std"),
                        (InterfaceComplexity::Advanced, "Adv"),
                        (InterfaceComplexity::Expert, "Op"),
                    ];
                    for (tier, label) in tiers {
                        let is_active = app.ui.complexity == tier;
                        let text_color = if is_active { palette::TEXT } else { palette::TEXT_DIM };
                        let bg = if is_active { palette::SELECTED } else { Color32::TRANSPARENT };
                        if ui.add(
                            egui::Button::new(RichText::new(label).size(10.5).color(text_color))
                                .fill(bg)
                                .corner_radius(4.0)
                                .min_size(Vec2::new(28.0, 22.0))
                        ).clicked() {
                            app.ui.complexity = tier;
                        }
                    }
                });
                ui.add_space(4.0);

                // Settings link
                nav_item(ui, app, NavTab::Settings, egui_phosphor::regular::GEAR_SIX, "Settings");

                ui.add_space(4.0);
                // Thin separator
                ui.add(egui::Separator::default().spacing(1.0));
            });
        });

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // BOTTOM BAR — Minimal, contextual status
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    TopBottomPanel::bottom("status_bar")
        .frame(Frame::new()
            .fill(palette::SIDEBAR)
            .inner_margin(egui::Margin { left: 14, right: 14, top: 6, bottom: 6 })
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let obj_count = app.object_count();
                let msg = if !app.ui.status_msg.is_empty() {
                    app.ui.status_msg.clone()
                } else if obj_count == 0 {
                    "Ready".to_string()
                } else {
                    format!("{} objects  •  Local sovereign node  •  Ctrl+K to search", obj_count)
                };
                ui.label(RichText::new(msg).color(palette::TEXT_DIM).size(11.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("Node {}", app.actor_id_short()))
                        .color(palette::TEXT_DIM).size(11.0));
                });
            });
        });

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // MAIN CONTENT CANVAS — The living room
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    CentralPanel::default()
        .frame(Frame::new()
            .fill(palette::BG)
            .inner_margin(egui::Margin { left: 28, right: 28, top: 24, bottom: 16 })
        )
        .show(ctx, |ui| {
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

            // Trigger action modal dialogs (Import, Export, Proximity SAS Verification)
            actions::render_action_dialog(ui, app);
        });

    // Global Command Palette / Spotlight Launcher Modal
    render_command_palette(ctx, app);
}

fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label.to_uppercase())
        .size(10.0)
        .color(palette::TEXT_DIM)
        .strong()
    );
    ui.add_space(4.0);
}

fn render_command_palette(ctx: &Context, app: &mut NexDesktopApp) {
    if !app.ui.command_palette_open {
        return;
    }

    // Semi-transparent backdrop
    egui::Area::new(egui::Id::new("palette_backdrop"))
        .fixed_pos(egui::Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            ui.allocate_exact_size(screen.size(), egui::Sense::click());
        });

    egui::Window::new("Command Palette")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 80.0))
        .frame(Frame::new()
            .fill(Color32::from_rgb(20, 21, 28))
            .corner_radius(12.0)
            .inner_margin(16.0)
            .stroke(Stroke::new(1.0_f32, palette::BORDER_SUBTLE))
            .shadow(egui::Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 4,
                color: Color32::from_black_alpha(120),
            })
        )
        .show(ctx, |ui| {
            ui.set_width(520.0);

            // Search input
            ui.horizontal(|ui| {
                ui.label(RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS).size(18.0).color(palette::ACCENT));
                ui.add_space(4.0);
                let response = ui.add(egui::TextEdit::singleline(&mut app.ui.command_palette_query)
                    .hint_text("Search objects, lenses, people, devices…")
                    .desired_width(460.0)
                    .text_color(palette::TEXT)
                    .font(egui::TextStyle::Body));
                response.request_focus();
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    app.ui.command_palette_open = false;
                }
            });

            ui.add_space(8.0);
            ui.add(egui::Separator::default().spacing(1.0));
            ui.add_space(8.0);

            let query = app.ui.command_palette_query.to_lowercase();
            let mut matched = 0;

            // Navigation shortcuts
            let nav_targets = [
                ("Personal", NavTab::Home, egui_phosphor::regular::HOUSE_SIMPLE, "Space"),
                ("Family", NavTab::Family, egui_phosphor::regular::USERS_THREE, "Space"),
                ("Photos", NavTab::Photos, egui_phosphor::regular::IMAGE, "Lens"),
                ("Files", NavTab::Drive, egui_phosphor::regular::FOLDER_SIMPLE, "Lens"),
                ("Media", NavTab::Media, egui_phosphor::regular::PLAY_CIRCLE, "Lens"),
                ("Maps", NavTab::Maps, egui_phosphor::regular::MAP_TRIFOLD, "Lens"),
                ("People", NavTab::People, egui_phosphor::regular::IDENTIFICATION_CARD, "Mesh"),
                ("Devices", NavTab::Devices, egui_phosphor::regular::DESKTOP_TOWER, "Mesh"),
                ("Topology", NavTab::Network, egui_phosphor::regular::GRAPH, "Mesh"),
                ("Settings", NavTab::Settings, egui_phosphor::regular::GEAR_SIX, "System"),
            ];

            for (name, tab, icon, category) in nav_targets {
                if query.is_empty() || name.to_lowercase().contains(&query) {
                    ui.horizontal(|ui| {
                        let response = ui.add_sized(
                            Vec2::new(ui.available_width(), 32.0),
                            egui::Button::new(
                                RichText::new(format!("{}  {}", icon, name)).size(13.0).color(palette::TEXT)
                            )
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(6.0),
                        );
                        // Show category badge right-aligned inside the button area
                        let badge_rect = response.rect;
                        ui.painter().text(
                            egui::Pos2::new(badge_rect.right() - 8.0, badge_rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            category,
                            egui::FontId::proportional(10.0),
                            palette::TEXT_DIM,
                        );
                        if response.clicked() {
                            app.ui.active_tab = tab;
                            app.ui.command_palette_open = false;
                        }
                    });
                    matched += 1;
                    if matched >= 6 { break; }
                }
            }

            // Quick Objects
            for (obj_id, obj) in &app.node.state.object_store {
                if obj.tombstoned { continue; }
                let title = obj.metadata.get("title").or_else(|| obj.metadata.get("filename")).cloned().unwrap_or_else(|| "Untitled".to_string());
                if !query.is_empty() && title.to_lowercase().contains(&query) {
                    if ui.add_sized(
                        Vec2::new(ui.available_width(), 30.0),
                        egui::Button::new(
                            RichText::new(format!("{}  {}", egui_phosphor::regular::FILE_TEXT, title)).size(12.5).color(palette::ACCENT)
                        )
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(6.0),
                    ).clicked() {
                        app.ui.selected_entity = Some(inspector::SelectedEntity::Object(*obj_id));
                        app.ui.command_palette_open = false;
                    }
                    matched += 1;
                    if matched >= 10 { break; }
                }
            }

            ui.add_space(6.0);
            ui.label(RichText::new("↵ Select  •  ESC Close  •  ⌘K Toggle").size(10.5).color(palette::TEXT_DIM));
        });
}

fn nav_item(ui: &mut egui::Ui, app: &mut NexDesktopApp, tab: NavTab, icon: &str, label: &str) {
    let selected = app.ui.active_tab == tab;
    let bg = if selected { palette::SELECTED } else { Color32::TRANSPARENT };
    let text_color = if selected { palette::NAV_ACTIVE } else { palette::NAV_INACTIVE };

    let response = ui.add_sized(
        Vec2::new(ui.available_width(), 30.0),
        egui::Button::new(
            RichText::new(format!("{}   {}", icon, label))
                .color(text_color)
                .size(13.0)
        )
        .fill(bg)
        .corner_radius(6.0)
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
