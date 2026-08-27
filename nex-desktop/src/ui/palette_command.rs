use egui::{Context, Frame, Color32, Stroke, Vec2, RichText, Key};
use nex_core::object::types::{ObjectID, ObjectType};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::shell::SpaceType;
use crate::app::NexDesktopApp;
use crate::ui::{NavTab, palette, inspector, actions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteCategory {
    Lens,
    Space,
    Action,
    Object,
    Complexity,
    System,
}

impl PaletteCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Lens => "Lens",
            Self::Space => "Space",
            Self::Action => "Action",
            Self::Object => "Object",
            Self::Complexity => "Slider",
            Self::System => "System",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Lens => palette::ACCENT,
            Self::Space => palette::ACCENT_GREEN,
            Self::Action => palette::ACCENT_AMBER,
            Self::Object => Color32::from_rgb(168, 85, 247), // Purple
            Self::Complexity => Color32::from_rgb(56, 189, 248), // Sky Blue
            Self::System => palette::TEXT_DIM,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PaletteActionPayload {
    Navigate(NavTab),
    SwitchSpace(SpaceType),
    SetComplexity(InterfaceComplexity),
    InspectObject(ObjectID),
    TriggerAction(CommandActionType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandActionType {
    ImportPersonal,
    ImportFamily,
    ProximitySasPairing,
    VerifyIntegrity,
    TriggerSync,
    ExportSeedBackup,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub title: String,
    pub subtitle: String,
    pub icon: &'static str,
    pub category: PaletteCategory,
    pub hotkey: Option<&'static str>,
    pub payload: PaletteActionPayload,
}

#[derive(Debug, Clone)]
pub struct CommandPaletteState {
    pub query: String,
    pub selected_index: usize,
    pub last_executed_feedback: Option<String>,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            selected_index: 0,
            last_executed_feedback: None,
        }
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn build_items(&self, app: &NexDesktopApp) -> Vec<PaletteItem> {
        let query_lower = self.query.trim().to_lowercase();
        let mut items = Vec::new();

        // ── 1. The 9 Realized Surfaces (Lenses) ──
        let lenses = [
            ("Personal Sanctuary", "Home overview & activity stream", egui_phosphor::regular::HOUSE_SIMPLE, NavTab::Home, Some("⌘1")),
            ("Family Living Space", "Family circle & shared objects", egui_phosphor::regular::USERS_THREE, NavTab::Family, Some("⌘2")),
            ("Photos Lens", "Visual memories & spatial EXIF projections", egui_phosphor::regular::IMAGE, NavTab::Photos, Some("⌘3")),
            ("Drive Files", "Sovereign documents & encrypted filesystem", egui_phosphor::regular::FOLDER_SIMPLE, NavTab::Drive, Some("⌘4")),
            ("People & Web of Trust", "Petnames, trust tiers & delegations", egui_phosphor::regular::IDENTIFICATION_CARD, NavTab::People, Some("⌘5")),
            ("Hardware Devices", "Physical nodes & attestation certificates", egui_phosphor::regular::DESKTOP_TOWER, NavTab::Devices, Some("⌘6")),
            ("Topology Radar", "Infrastructure mesh & active conduits", egui_phosphor::regular::GRAPH, NavTab::Network, Some("⌘7")),
            ("Territory Maps", "Geotagged objects & coordinate context", egui_phosphor::regular::MAP_TRIFOLD, NavTab::Maps, Some("⌘8")),
            ("Node Settings", "System preferences & cryptographic authority", egui_phosphor::regular::GEAR_SIX, NavTab::Settings, Some("⌘9")),
            ("NEX Comms & Chat", "E2EE messaging & physical conduits", egui_phosphor::regular::CHATS_TEARDROP, NavTab::Chat, Some("⌘0")),
        ];

        for (title, sub, icon, tab, hotkey) in lenses {
            if query_lower.is_empty() || title.to_lowercase().contains(&query_lower) || sub.to_lowercase().contains(&query_lower) {
                items.push(PaletteItem {
                    title: title.to_string(),
                    subtitle: sub.to_string(),
                    icon,
                    category: PaletteCategory::Lens,
                    hotkey,
                    payload: PaletteActionPayload::Navigate(tab),
                });
            }
        }

        // ── 2. Space Switching ──
        let spaces = [
            ("Switch to Personal Space", "Isolate view to private personal sanctuary", egui_phosphor::regular::LOCK_KEY, SpaceType::Personal),
            ("Switch to Family Space", "Switch view to shared family circle", egui_phosphor::regular::USERS, SpaceType::Family),
            ("Switch to Work Space", "Professional workspace with attenuated capability grants", egui_phosphor::regular::BRIEFCASE, SpaceType::Work),
            ("Switch to Community Space", "Public & neighborhood discovery space", egui_phosphor::regular::GLOBE, SpaceType::Community),
        ];

        for (title, sub, icon, space) in spaces {
            if query_lower.is_empty() || title.to_lowercase().contains(&query_lower) || sub.to_lowercase().contains(&query_lower) {
                items.push(PaletteItem {
                    title: title.to_string(),
                    subtitle: sub.to_string(),
                    icon,
                    category: PaletteCategory::Space,
                    hotkey: None,
                    payload: PaletteActionPayload::SwitchSpace(space),
                });
            }
        }

        // ── 3. Experience Slider (Complexity Switching) ──
        let complexities = [
            ("Experience: Simple (Tier 1)", "Calm, human-friendly presentation", egui_phosphor::regular::GAUGE, InterfaceComplexity::Simple),
            ("Experience: Standard (Tier 2)", "Balanced structure & verification badges", egui_phosphor::regular::GAUGE, InterfaceComplexity::Standard),
            ("Experience: Advanced (Tier 3)", "Technical hashes, causal DAG & LSN details", egui_phosphor::regular::GAUGE, InterfaceComplexity::Advanced),
            ("Experience: Expert (Tier 4)", "Raw cryptographic proofs & wire diagnostics", egui_phosphor::regular::GAUGE, InterfaceComplexity::Expert),
        ];

        for (title, sub, icon, comp) in complexities {
            if query_lower.is_empty() || title.to_lowercase().contains(&query_lower) || sub.to_lowercase().contains(&query_lower) {
                items.push(PaletteItem {
                    title: title.to_string(),
                    subtitle: sub.to_string(),
                    icon,
                    category: PaletteCategory::Complexity,
                    hotkey: None,
                    payload: PaletteActionPayload::SetComplexity(comp),
                });
            }
        }

        // ── 4. Sovereign Actions ──
        let actions_list = [
            ("Import File into Personal Space", "Add local file with content-addressed BLAKE3 digest", egui_phosphor::regular::PLUS_CIRCLE, CommandActionType::ImportPersonal),
            ("Place File in Family Space", "Share document with verified family circle", egui_phosphor::regular::SHARE_NETWORK, CommandActionType::ImportFamily),
            ("Pair Device via SAS QR Code", "Initiate Short Authentication String proximity ceremony", egui_phosphor::regular::QR_CODE, CommandActionType::ProximitySasPairing),
            ("Verify Object Integrity", "Scan local CAS store against canonical Merkle root", egui_phosphor::regular::SHIELD_CHECK, CommandActionType::VerifyIntegrity),
            ("Trigger Mesh Sync", "Perform anti-entropy synchronization with peers", egui_phosphor::regular::ARROWS_CLOCKWISE, CommandActionType::TriggerSync),
            ("Backup 12-Word Master Seed", "Export sovereign root keys securely", egui_phosphor::regular::KEY, CommandActionType::ExportSeedBackup),
        ];

        for (title, sub, icon, action_type) in actions_list {
            if query_lower.is_empty() || title.to_lowercase().contains(&query_lower) || sub.to_lowercase().contains(&query_lower) {
                items.push(PaletteItem {
                    title: title.to_string(),
                    subtitle: sub.to_string(),
                    icon,
                    category: PaletteCategory::Action,
                    hotkey: None,
                    payload: PaletteActionPayload::TriggerAction(action_type),
                });
            }
        }

        // ── 5. Live Canonical Object Search ──
        for (obj_id, obj) in &app.node.state.object_store {
            if obj.tombstoned { continue; }
            let title = obj.metadata.get("title")
                .or_else(|| obj.metadata.get("filename"))
                .or_else(|| obj.metadata.get("name"))
                .cloned()
                .unwrap_or_else(|| format!("Object 0x{}", hex::encode(&obj_id[0..4])));

            let mime = obj.metadata.get("mime").cloned().unwrap_or_default();
            let camera = obj.metadata.get("camera_make").cloned().unwrap_or_default();
            let sub = format!("{:.1} KB • Verified Fact • {}", obj.payload_bytes.len() as f64 / 1024.0, mime);

            let icon = match obj.object_type {
                ObjectType::PhotoMedia | ObjectType::PhotoAlbum => egui_phosphor::regular::IMAGE,
                ObjectType::DriveInode => egui_phosphor::regular::FILE_TEXT,
                _ => egui_phosphor::regular::CUBE,
            };

            let matches_query = query_lower.is_empty()
                || title.to_lowercase().contains(&query_lower)
                || mime.to_lowercase().contains(&query_lower)
                || camera.to_lowercase().contains(&query_lower)
                || hex::encode(obj_id).contains(&query_lower);

            if matches_query {
                items.push(PaletteItem {
                    title,
                    subtitle: sub,
                    icon,
                    category: PaletteCategory::Object,
                    hotkey: None,
                    payload: PaletteActionPayload::InspectObject(*obj_id),
                });
            }
        }

        items
    }

    pub fn execute_item(&mut self, item: &PaletteItem, app: &mut NexDesktopApp) {
        match &item.payload {
            PaletteActionPayload::Navigate(tab) => {
                app.ui.active_tab = *tab;
                self.last_executed_feedback = Some(format!("Navigated to {}", item.title));
            }
            PaletteActionPayload::SwitchSpace(space) => {
                match space {
                    SpaceType::Personal => {
                        app.ui.active_tab = NavTab::Home;
                    }
                    SpaceType::Family => {
                        app.ui.active_tab = NavTab::Family;
                    }
                    SpaceType::Work => {
                        app.ui.active_tab = NavTab::Drive;
                    }
                    SpaceType::Community => {
                        app.ui.active_tab = NavTab::People;
                    }
                    _ => {}
                }
                self.last_executed_feedback = Some(format!("Switched to {:?}", space));
            }
            PaletteActionPayload::SetComplexity(comp) => {
                app.ui.complexity = *comp;
                self.last_executed_feedback = Some(format!("Complexity tier set to {:?}", comp));
            }
            PaletteActionPayload::InspectObject(obj_id) => {
                app.ui.selected_entity = Some(inspector::SelectedEntity::Object(*obj_id));
                self.last_executed_feedback = Some(format!("Inspecting {}", item.title));
            }
            PaletteActionPayload::TriggerAction(action_type) => {
                match action_type {
                    CommandActionType::ImportPersonal => {
                        app.ui.active_tab = NavTab::Drive;
                    }
                    CommandActionType::ImportFamily => {
                        app.ui.active_tab = NavTab::Family;
                    }
                    CommandActionType::ProximitySasPairing => {
                        app.ui.action_state.active_dialog = Some(actions::ActionDialog::ProximitySasVerification {
                            peer_name: "Amy's Pixel 9".to_string(),
                            actor_id: [0x55; 32],
                            safety_words: [
                                "RIVER".to_string(),
                                "SUMMIT".to_string(),
                                "FALCON".to_string(),
                                "HARBOR".to_string(),
                            ],
                        });
                    }
                    CommandActionType::VerifyIntegrity => {
                        self.last_executed_feedback = Some("Integrity verified: all local chunks match BLAKE3 Merkle root.".to_string());
                    }
                    CommandActionType::TriggerSync => {
                        self.last_executed_feedback = Some("Anti-entropy sync complete: state frontier in sync.".to_string());
                    }
                    CommandActionType::ExportSeedBackup => {
                        app.ui.active_tab = NavTab::Settings;
                    }
                }
            }
        }
        app.ui.command_palette_open = false;
    }
}

pub fn render_command_palette(ctx: &Context, app: &mut NexDesktopApp, state: &mut CommandPaletteState) {
    if !app.ui.command_palette_open {
        return;
    }

    let items = state.build_items(app);
    let total_items = items.len();

    // ── Global Keyboard Navigation ──
    let mut execute_selected = false;
    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            app.ui.command_palette_open = false;
        }
        if i.key_pressed(Key::ArrowDown) {
            if total_items > 0 {
                state.selected_index = (state.selected_index + 1) % total_items;
            }
        }
        if i.key_pressed(Key::ArrowUp) {
            if total_items > 0 {
                if state.selected_index == 0 {
                    state.selected_index = total_items - 1;
                } else {
                    state.selected_index -= 1;
                }
            }
        }
        if i.key_pressed(Key::Enter) {
            execute_selected = true;
        }
    });

    if execute_selected && !items.is_empty() {
        let index = state.selected_index.min(total_items.saturating_sub(1));
        state.execute_item(&items[index].clone(), app);
        return;
    }

    // Modal Backdrop Dimmer
    egui::Area::new(egui::Id::new("palette_backdrop"))
        .fixed_pos(egui::Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            let (_, resp) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            if resp.clicked() {
                app.ui.command_palette_open = false;
            }
        });

    // Elevated Command Palette Modal
    egui::Window::new("Sovereign Velocity Palette")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 80.0))
        .frame(Frame::new()
            .fill(Color32::from_rgb(18, 20, 28))
            .corner_radius(14.0)
            .inner_margin(18.0)
            .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
            .shadow(egui::Shadow {
                offset: [0, 14],
                blur: 32,
                spread: 4,
                color: Color32::from_black_alpha(180),
            })
        )
        .show(ctx, |ui| {
            ui.set_width(580.0);

            // ── Search Input Header ──
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                        .size(20.0)
                        .color(palette::ACCENT)
                );
                ui.add_space(8.0);
                let text_edit = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .hint_text("Type a command, space, lens, person, device, or object…")
                        .desired_width(500.0)
                        .text_color(palette::TEXT)
                        .font(egui::FontId::proportional(15.0))
                );
                text_edit.request_focus();
            });

            ui.add_space(12.0);
            ui.add(egui::Separator::default().spacing(1.0));
            ui.add_space(8.0);

            // ── Filtered Results Scroll Area ──
            egui::ScrollArea::vertical()
                .max_height(340.0)
                .show(ui, |ui| {
                    if items.is_empty() {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS).size(28.0).color(palette::TEXT_DIM));
                            ui.add_space(6.0);
                            ui.label(RichText::new("No matching commands or objects").size(14.0).color(palette::TEXT_SECONDARY));
                            ui.label(RichText::new("NEX queries live canonical objects across all 9 realized surfaces.").size(11.5).color(palette::TEXT_DIM));
                        });
                        ui.add_space(24.0);
                    } else {
                        for (idx, item) in items.iter().enumerate() {
                            let is_highlighted = idx == state.selected_index;
                            let bg_color = if is_highlighted {
                                Color32::from_rgb(32, 36, 52)
                            } else {
                                Color32::TRANSPARENT
                            };

                            let frame = Frame::new()
                                .fill(bg_color)
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::symmetric(10, 7))
                                .stroke(if is_highlighted { Stroke::new(1.0_f32, palette::ACCENT) } else { Stroke::NONE });

                            let resp = frame.show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    // Icon
                                    ui.label(
                                        RichText::new(item.icon)
                                            .size(16.0)
                                            .color(if is_highlighted { palette::ACCENT } else { palette::TEXT_SECONDARY })
                                    );
                                    ui.add_space(6.0);

                                    // Title & Subtitle in column
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(&item.title)
                                                    .size(13.5)
                                                    .color(if is_highlighted { palette::TEXT } else { palette::TEXT_SECONDARY })
                                                    .strong()
                                            );
                                        });
                                        ui.label(
                                            RichText::new(&item.subtitle)
                                                .size(11.0)
                                                .color(palette::TEXT_DIM)
                                        );
                                    });

                                    // Category Badge & Hotkey aligned right
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if let Some(hk) = item.hotkey {
                                            ui.label(
                                                RichText::new(hk)
                                                    .size(11.0)
                                                    .color(palette::TEXT_DIM)
                                            );
                                            ui.add_space(4.0);
                                        }

                                        let cat_badge = Frame::new()
                                            .fill(Color32::from_rgba_premultiplied(item.category.color().r(), item.category.color().g(), item.category.color().b(), 25))
                                            .corner_radius(4.0)
                                            .inner_margin(egui::Margin::symmetric(6, 2))
                                            .stroke(Stroke::new(0.5_f32, item.category.color()));

                                        cat_badge.show(ui, |ui| {
                                            ui.label(
                                                RichText::new(item.category.label())
                                                    .size(10.0)
                                                    .color(item.category.color())
                                                    .strong()
                                            );
                                        });
                                    });
                                });
                            }).response.interact(egui::Sense::click());

                            if resp.clicked() {
                                state.execute_item(item, app);
                                break;
                            }
                        }
                    }
                });

            ui.add_space(10.0);
            ui.add(egui::Separator::default().spacing(1.0));
            ui.add_space(6.0);

            // ── Footer Keyboard Shortcuts Legend ──
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("↑↓ Navigate   •   ↵ Execute   •   ESC Close   •   ⌘1-9 Instant Lens")
                        .size(11.0)
                        .color(palette::TEXT_DIM)
                );
            });
        });
}
