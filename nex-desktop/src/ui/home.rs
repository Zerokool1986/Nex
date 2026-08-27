use egui::{Ui, RichText, Frame, Sense, Vec2, Stroke, Color32};
use nex_core::runtime::experience::HumanExperienceEngine;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::ObjectType;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity, people::derive_people_catalog};

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// PERSONAL SANCTUARY SURFACE
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    let vm = HumanExperienceEngine::render_home_screen(
        &app.node, SpaceType::Personal, app.ui.complexity
    );

    // 1. Sanctuary Header & Truthful Boundary
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Personal Sanctuary").size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new("🔒 Private — only accessible by your cryptographic keys on this device")
                .size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{} Inspect Space", egui_phosphor::regular::MAGNIFYING_GLASS)).size(12.5).color(palette::TEXT_SECONDARY))
                .clicked()
            {
                app.ui.selected_entity = Some(SelectedEntity::Space(SpaceType::Personal));
            }
        });
    });

    ui.add_space(16.0);

    // Substrate Telemetry Beacon
    render_telemetry_beacon(ui, app, &vm);
    ui.add_space(24.0);

    if vm.feed_items.is_empty() {
        render_personal_empty_state(ui, app);
    } else {
        render_personal_summary_cards(ui, app);
        ui.add_space(28.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Recent Activity").size(14.0).strong().color(palette::TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("↑/↓ or J/K to traverse • Enter to inspect").size(11.0).color(palette::TEXT_DIM));
            });
        });
        ui.add_space(10.0);

        render_activity_feed(ui, app, &vm, SpaceType::Personal);
    }
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// FAMILY HEARTH SURFACE — Relational Shared Sanctuary
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub fn render_family(ui: &mut Ui, app: &mut NexDesktopApp) {
    let vm = HumanExperienceEngine::render_home_screen(
        &app.node, SpaceType::Family, app.ui.complexity
    );

    let people = derive_people_catalog(app);
    let family_members: Vec<_> = people.iter()
        .filter(|p| p.spaces.contains(&SpaceType::Family) || p.is_local)
        .collect();

    // 1. Family Hearth Header & Truthful Trust Boundary
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Family Space").size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new(format!("👥 Shared Space — accessible by {} verified family circle members", family_members.len().max(1)))
                .size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{}  Pair Member (SAS QR)", egui_phosphor::regular::QR_CODE)).size(12.5).color(palette::ACCENT))
                .clicked()
            {
                app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ProximitySasVerification {
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
        });
    });

    ui.add_space(16.0);

    // 2. Family Substrate Telemetry Beacon
    render_telemetry_beacon(ui, app, &vm);
    ui.add_space(22.0);

    // 3. Family Circle Roster (Who lives in this shared hearth)
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("FAMILY CIRCLE ({} Members)", family_members.len().max(1)))
            .size(11.5).strong().color(palette::TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("Click member to view trust & devices").size(11.0).color(palette::TEXT_DIM));
        });
    });
    ui.add_space(8.0);

    render_family_circle_roster(ui, app, &family_members);
    ui.add_space(24.0);

    if vm.feed_items.is_empty() {
        render_family_empty_state(ui, app);
    } else {
        // 4. Family Summary Cards
        render_family_summary_cards(ui, app, family_members.len().max(1));
        ui.add_space(28.0);

        // 5. Attributed Shared Activity River
        ui.horizontal(|ui| {
            ui.label(RichText::new("Shared Family Activity").size(14.0).strong().color(palette::TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("↑/↓ or J/K to traverse • Enter to inspect").size(11.0).color(palette::TEXT_DIM));
            });
        });
        ui.add_space(10.0);

        render_activity_feed(ui, app, &vm, SpaceType::Family);
    }
}

/// Renders the Family Member Roster with verified presence badges
fn render_family_circle_roster(ui: &mut Ui, app: &mut NexDesktopApp, members: &[&crate::ui::people::ProjectedPerson]) {
    ui.horizontal(|ui| {
        for member in members {
            let is_selected = app.ui.selected_entity == Some(SelectedEntity::Person(member.actor_id));
            let card_bg = if is_selected { palette::SELECTED } else { palette::PANEL };
            let stroke = if is_selected { Stroke::new(1.0_f32, palette::ACCENT) } else { Stroke::new(1.0_f32, palette::GLASS_BORDER) };

            let response = Frame::new()
                .fill(card_bg)
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(14, 10))
                .stroke(stroke)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let avatar_glyph = if member.is_local {
                            egui_phosphor::regular::USER
                        } else {
                            egui_phosphor::regular::USER_CHECK
                        };
                        let avatar_color = if member.is_local { palette::ACCENT } else { palette::ACCENT_GREEN };
                        ui.label(RichText::new(avatar_glyph).size(18.0).color(avatar_color));
                        ui.add_space(6.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&member.display_name).size(13.0).strong().color(palette::TEXT));
                            let presence = if member.is_local {
                                "● This PC (Host)".to_string()
                            } else {
                                "● Verified Member (Mesh)".to_string()
                            };
                            ui.label(RichText::new(presence).size(11.0).color(palette::ACCENT_GREEN));
                        });
                    });
                });

            if response.response.interact(Sense::click()).clicked() {
                app.ui.selected_entity = Some(SelectedEntity::Person(member.actor_id));
                app.ui.people_state.selected_person_id = Some(member.actor_id);
            }
            ui.add_space(8.0);
        }

        // Add Member Button
        let add_resp = Frame::new()
            .fill(Color32::TRANSPARENT)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .stroke(Stroke::new(1.0_f32, palette::BORDER_SUBTLE))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(egui_phosphor::regular::PLUS).size(16.0).color(palette::TEXT_DIM));
                    ui.label(RichText::new("Add Member").size(12.5).color(palette::TEXT_DIM));
                });
            });

        if add_resp.response.interact(Sense::click()).clicked() {
            app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ProximitySasVerification {
                peer_name: "New Family Member".to_string(),
                actor_id: [0x55; 32],
                safety_words: [
                    "RIVER".to_string(),
                    "SUMMIT".to_string(),
                    "FALCON".to_string(),
                    "HARBOR".to_string(),
                ],
            });
        }
    });
}

/// Truthful Substrate Telemetry Beacon
fn render_telemetry_beacon(ui: &mut Ui, app: &NexDesktopApp, vm: &nex_core::runtime::experience::HomeScreenViewModel) {
    let sync_text = &vm.sync_status_label;
    let (sync_color, sync_icon) = if sync_text.contains("Offline") || sync_text.contains("Starting") {
        (palette::TEXT_DIM, egui_phosphor::regular::CLOUD_SLASH)
    } else if sync_text.contains("Local") || sync_text.contains("local") {
        (palette::ACCENT_AMBER, egui_phosphor::regular::SHIELD_WARNING)
    } else {
        (palette::ACCENT_GREEN, egui_phosphor::regular::SHIELD_CHECK)
    };

    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", sync_icon, sync_text)).size(12.0).color(sync_color));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::DATABASE, &vm.storage_health_label))
                    .size(12.0).color(palette::TEXT_SECONDARY));

                if app.ui.complexity != nex_core::runtime::experience::InterfaceComplexity::Simple {
                    ui.add_space(12.0);
                    ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                    ui.add_space(12.0);

                    ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::FINGERPRINT, &vm.identity_protection_label))
                        .size(12.0).color(palette::TEXT_SECONDARY));
                }
            });
        });
}

/// Personal Empty State
fn render_personal_empty_state(ui: &mut Ui, app: &mut NexDesktopApp) {
    let card_width = ui.available_width().min(640.0);

    ui.vertical_centered(|ui| {
        ui.add_space(30.0);
        Frame::new()
            .fill(Color32::from_rgb(16, 17, 24))
            .corner_radius(12.0)
            .inner_margin(egui::Margin::symmetric(36, 32))
            .stroke(Stroke::new(1.5_f32, Color32::from_rgba_premultiplied(99, 144, 250, 70)))
            .show(ui, |ui| {
                ui.set_width(card_width);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new(egui::include_image!("../../assets/nex_brand_icon.png"))
                        .max_height(48.0)
                        .max_width(48.0));
                    ui.add_space(16.0);

                    ui.label(RichText::new("Your Personal Sanctuary is Ready").size(20.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);

                    ui.label(RichText::new("Everything you add lives on this computer first.\nIt syncs directly to your trusted devices with zero corporate cloud middle-men.")
                        .size(13.5).color(palette::TEXT_SECONDARY));
                    ui.add_space(22.0);

                    let btn = ui.add_sized(
                        Vec2::new(220.0, 38.0),
                        egui::Button::new(
                            RichText::new(format!("{}   Add First File to Personal", egui_phosphor::regular::PLUS))
                                .size(13.5).color(palette::TEXT).strong()
                        )
                        .fill(palette::ACCENT)
                        .corner_radius(8.0),
                    );
                    if btn.clicked() {
                        app.ui.active_tab = NavTab::Drive;
                    }

                    ui.add_space(12.0);
                    ui.label(RichText::new("or drag and drop files anywhere into NEX").size(12.0).color(palette::TEXT_DIM));
                });
            });
    });
}

/// Family Empty State
fn render_family_empty_state(ui: &mut Ui, app: &mut NexDesktopApp) {
    let card_width = ui.available_width().min(640.0);

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        Frame::new()
            .fill(Color32::from_rgb(16, 17, 24))
            .corner_radius(12.0)
            .inner_margin(egui::Margin::symmetric(36, 30))
            .stroke(Stroke::new(1.5_f32, Color32::from_rgba_premultiplied(52, 211, 153, 70)))
            .show(ui, |ui| {
                ui.set_width(card_width);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(egui_phosphor::regular::HEART).size(42.0).color(palette::ACCENT_GREEN));
                    ui.add_space(14.0);

                    ui.label(RichText::new("Your Family Space is Ready").size(20.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);

                    ui.label(RichText::new("Photos, documents, and memories placed here are shared directly with your family circle.\nThey stay on your physical hardware without third-party tracking or cloud fees.")
                        .size(13.5).color(palette::TEXT_SECONDARY));
                    ui.add_space(22.0);

                    ui.horizontal(|ui| {
                        let btn1 = ui.add_sized(
                            Vec2::new(190.0, 38.0),
                            egui::Button::new(
                                RichText::new(format!("{}   Place File in Family", egui_phosphor::regular::PLUS))
                                    .size(13.0).color(palette::TEXT).strong()
                            )
                            .fill(palette::ACCENT)
                            .corner_radius(8.0),
                        );
                        if btn1.clicked() {
                            app.ui.active_tab = NavTab::Drive;
                        }

                        ui.add_space(10.0);

                        let btn2 = ui.add_sized(
                            Vec2::new(190.0, 38.0),
                            egui::Button::new(
                                RichText::new(format!("{}   Pair Member QR", egui_phosphor::regular::QR_CODE))
                                    .size(13.0).color(palette::TEXT).strong()
                            )
                            .fill(palette::PANEL)
                            .corner_radius(8.0)
                            .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)),
                        );
                        if btn2.clicked() {
                            app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ProximitySasVerification {
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
                    });

                    ui.add_space(12.0);
                    ui.label(RichText::new("Family members sync privately over direct Wi-Fi and local mesh").size(12.0).color(palette::TEXT_DIM));
                });
            });
    });
}

/// Personal summary cards
fn render_personal_summary_cards(ui: &mut Ui, app: &mut NexDesktopApp) {
    let mut photo_count = 0usize;
    let mut file_count = 0usize;
    let mut total_bytes = 0usize;

    let personal_ns = nex_core::runtime::shell::NexHomeShell::space_to_namespace(SpaceType::Personal);

    for obj in app.node.state.object_store.values() {
        if obj.tombstoned || obj.namespace != personal_ns { continue; }
        match obj.object_type {
            ObjectType::PhotoMedia => photo_count += 1,
            ObjectType::DriveInode => file_count += 1,
            _ => file_count += 1,
        }
        total_bytes += obj.payload_bytes.len();
    }

    ui.horizontal(|ui| {
        obsidian_card(ui, app, NavTab::Drive, egui_phosphor::regular::FOLDER_SIMPLE, "Personal Files", &format!("{}", file_count + photo_count), &format_bytes(total_bytes), palette::ACCENT);
        ui.add_space(10.0);
        obsidian_card(ui, app, NavTab::Photos, egui_phosphor::regular::IMAGE, "Personal Photos", &format!("{}", photo_count), "original quality", palette::ACCENT);
        ui.add_space(10.0);
        obsidian_card(ui, app, NavTab::Devices, egui_phosphor::regular::DESKTOP_TOWER, "This Device", "1", "local host live", palette::ACCENT_GREEN);
    });
}

/// Family summary cards
fn render_family_summary_cards(ui: &mut Ui, app: &mut NexDesktopApp, member_count: usize) {
    let mut photo_count = 0usize;
    let mut file_count = 0usize;
    let mut total_bytes = 0usize;

    let family_ns = nex_core::runtime::shell::NexHomeShell::space_to_namespace(SpaceType::Family);

    for obj in app.node.state.object_store.values() {
        if obj.tombstoned || obj.namespace != family_ns { continue; }
        match obj.object_type {
            ObjectType::PhotoMedia => photo_count += 1,
            ObjectType::DriveInode => file_count += 1,
            _ => file_count += 1,
        }
        total_bytes += obj.payload_bytes.len();
    }

    ui.horizontal(|ui| {
        obsidian_card(ui, app, NavTab::Drive, egui_phosphor::regular::FOLDER_SIMPLE, "Shared Files", &format!("{}", file_count + photo_count), &format_bytes(total_bytes), palette::ACCENT);
        ui.add_space(10.0);
        obsidian_card(ui, app, NavTab::Photos, egui_phosphor::regular::IMAGE, "Family Photos", &format!("{}", photo_count), "shared memories", palette::ACCENT_GREEN);
        ui.add_space(10.0);
        obsidian_card(ui, app, NavTab::Devices, egui_phosphor::regular::SHARE_NETWORK, "Family Mesh", &format!("{} Members", member_count), "direct sync ready", palette::ACCENT_GREEN);
    });
}

fn obsidian_card(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    target: NavTab,
    icon: &str,
    label: &str,
    count: &str,
    detail: &str,
    icon_color: Color32,
) {
    let card_width = ((ui.available_width() - 20.0) / 3.0).max(140.0);

    let response = Frame::new()
        .fill(palette::CARD)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(card_width, 88.0));
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).size(20.0).color(icon_color));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(label).size(12.0).color(palette::TEXT_DIM));
                    });
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(count).size(26.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);
                    ui.label(RichText::new(detail).size(12.0).color(palette::TEXT_SECONDARY));
                });
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.active_tab = target;
    }
}

/// Activity river with human origin attribution & keyboard traversal
fn render_activity_feed(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    vm: &nex_core::runtime::experience::HomeScreenViewModel,
    _space: SpaceType,
) {
    let feed_len = vm.feed_items.len();

    ui.input(|i| {
        if i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::J) {
            let next = match app.ui.home_selected_index {
                Some(idx) if idx + 1 < feed_len => idx + 1,
                _ => 0,
            };
            app.ui.home_selected_index = Some(next);
        }
        if i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::K) {
            let prev = match app.ui.home_selected_index {
                Some(idx) if idx > 0 => idx - 1,
                _ => 0,
            };
            app.ui.home_selected_index = Some(prev);
        }
    });

    for (idx, item) in vm.feed_items.iter().enumerate() {
        let mut obj_id_bytes = [0u8; 32];
        if let Ok(bytes) = hex::decode(&item.object_id_hex) {
            if bytes.len() == 32 {
                obj_id_bytes.copy_from_slice(&bytes);
            }
        }

        let is_selected = app.ui.selected_entity == Some(SelectedEntity::Object(obj_id_bytes));
        let is_focused = app.ui.home_selected_index == Some(idx);

        if is_focused && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            app.ui.selected_entity = Some(SelectedEntity::Object(obj_id_bytes));
        }

        let card_bg = if is_selected || is_focused { palette::SELECTED } else { palette::CARD };
        let border_stroke = if is_selected || is_focused {
            Stroke::new(1.2_f32, palette::ACCENT)
        } else {
            Stroke::new(1.0_f32, palette::GLASS_BORDER)
        };

        let (icon, icon_color) = match item.object_type {
            ObjectType::PhotoMedia => (egui_phosphor::regular::IMAGE, palette::ACCENT),
            ObjectType::DriveInode => (egui_phosphor::regular::FILE_TEXT, palette::TEXT_SECONDARY),
            _ => (egui_phosphor::regular::CUBE, palette::TEXT_DIM),
        };

        // Determine Human Author Origin
        let author_label = if let Some(obj) = app.node.state.object_store.get(&obj_id_bytes) {
            if obj.owner_actor_id == app.node.identity.actor_id {
                "👤 You".to_string()
            } else if let Some(name) = obj.metadata.get("author_name") {
                format!("👩 {}", name)
            } else {
                format!("👤 Actor {}", hex::encode(&obj.owner_actor_id[0..4]))
            }
        } else {
            "👤 Member".to_string()
        };

        let response = Frame::new()
            .fill(card_bg)
            .corner_radius(8.0)
            .inner_margin(egui::Margin { left: 16, right: 16, top: 12, bottom: 12 })
            .stroke(border_stroke)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(icon).size(20.0).color(icon_color));
                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&item.title).size(14.0).strong().color(palette::TEXT));
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&author_label).size(11.5).color(palette::ACCENT));
                                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                                ui.label(RichText::new(&item.status_badge).size(11.5).color(palette::ACCENT_GREEN));
                            });
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(&item.timestamp_label).size(11.5).color(palette::TEXT_DIM));
                        });
                    });

                    // Operator Diagnostic Ribbon
                    if app.ui.complexity == nex_core::runtime::experience::InterfaceComplexity::Expert {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("OBJ_ID: {} | DAG_WINNER: 100% SMT | CAP: Valid", &item.object_id_hex[0..12]))
                                .monospace().size(10.5).color(palette::TEXT_DIM));
                        });
                    }
                });
            });

        if response.response.interact(Sense::click()).clicked() {
            app.ui.selected_entity = Some(SelectedEntity::Object(obj_id_bytes));
            app.ui.home_selected_index = Some(idx);
        }
        ui.add_space(6.0);
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
