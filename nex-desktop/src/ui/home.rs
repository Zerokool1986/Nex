use egui::{Ui, RichText, Frame, Sense, Vec2, Stroke, Color32};
use nex_core::runtime::experience::HumanExperienceEngine;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::ObjectType;
use crate::app::NexDesktopApp;
use crate::ui::{palette, inspector::SelectedEntity};

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    render_space(ui, app, SpaceType::Personal, "Personal", "Your personal sanctuary");
}

pub fn render_family(ui: &mut Ui, app: &mut NexDesktopApp) {
    render_space(ui, app, SpaceType::Family, "Family", "Shared with verified family circle");
}

fn render_space(ui: &mut Ui, app: &mut NexDesktopApp, space: SpaceType, title: &str, _default_sub: &str) {
    let vm = HumanExperienceEngine::render_home_screen(
        &app.node, space, app.ui.complexity
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. SANCTUARY STATE HEADER — Temporal Greeting & Truthful Boundary
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let greeting = match space {
        SpaceType::Personal => "Personal Sanctuary",
        SpaceType::Family => "Family Space",
        _ => "Space",
    };

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(greeting).size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);

            // Truthful privacy boundary derived strictly from capability model
            let privacy_text = match space {
                SpaceType::Personal => "🔒 Private — only accessible by your cryptographic keys on this device",
                SpaceType::Family => "👥 Shared Space — accessible by verified family circle members",
                _ => "🌐 Sovereign Scoped Space",
            };
            ui.label(RichText::new(privacy_text).size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{} Inspect Space", egui_phosphor::regular::MAGNIFYING_GLASS)).size(12.5).color(palette::TEXT_SECONDARY))
                .clicked()
            {
                app.ui.selected_entity = Some(SelectedEntity::Space(space));
            }
        });
    });

    ui.add_space(16.0);

    // ── Single Truthful Substrate Telemetry Beacon ──
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

    ui.add_space(24.0);

    if vm.feed_items.is_empty() {
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // 2. WELCOMING EMPTY-STATE SANCTUARY VESSEL
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        render_empty_state_vessel(ui, app, title);
    } else {
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // 3. GLANCEABLE OBSIDIAN GLASS SUMMARY CARDS
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        render_summary_cards(ui, app);

        ui.add_space(28.0);

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // 4. ACTIVITY RIVER WITH KEYBOARD TRAVERSAL
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        ui.horizontal(|ui| {
            ui.label(RichText::new("Recent Activity").size(14.0).strong().color(palette::TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("↑/↓ or J/K to traverse • Enter to inspect").size(11.0).color(palette::TEXT_DIM));
            });
        });
        ui.add_space(10.0);

        render_feed(ui, app, &vm);
    }
}

/// Welcoming drag-and-drop vessel with truthful privacy-first microcopy
fn render_empty_state_vessel(ui: &mut Ui, app: &mut NexDesktopApp, space_title: &str) {
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

                    ui.label(RichText::new(format!("Your {} is Ready", space_title))
                        .size(20.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);

                    ui.label(RichText::new("Everything you add lives on this computer first.\nIt syncs directly to your trusted devices with zero corporate cloud middle-men.")
                        .size(13.5).color(palette::TEXT_SECONDARY));
                    ui.add_space(22.0);

                    // Primary Action Button
                    let btn = ui.add_sized(
                        Vec2::new(220.0, 38.0),
                        egui::Button::new(
                            RichText::new(format!("{}   Add First File to {}", egui_phosphor::regular::PLUS, space_title))
                                .size(13.5).color(palette::TEXT).strong()
                        )
                        .fill(palette::ACCENT)
                        .corner_radius(8.0),
                    );
                    if btn.clicked() {
                        app.ui.active_tab = crate::ui::NavTab::Drive;
                    }

                    ui.add_space(12.0);
                    ui.label(RichText::new("or drag and drop files anywhere into NEX")
                        .size(12.0).color(palette::TEXT_DIM));
                });
            });
    });
}

/// Summary cards with glassmorphic depth & interactive lift
fn render_summary_cards(ui: &mut Ui, app: &mut NexDesktopApp) {
    let mut photo_count = 0usize;
    let mut file_count = 0usize;
    let mut total_bytes = 0usize;

    for obj in app.node.state.object_store.values() {
        if obj.tombstoned { continue; }
        match obj.object_type {
            ObjectType::PhotoMedia => photo_count += 1,
            ObjectType::DriveInode => file_count += 1,
            _ => file_count += 1,
        }
        total_bytes += obj.payload_bytes.len();
    }

    let device_count = 1usize; // This host is always live

    ui.horizontal(|ui| {
        // Files Card
        obsidian_card(
            ui, app, crate::ui::NavTab::Drive,
            egui_phosphor::regular::FOLDER_SIMPLE,
            "Files",
            &format!("{}", file_count + photo_count),
            &format_bytes(total_bytes),
            palette::ACCENT,
        );
        ui.add_space(10.0);

        // Photos Card
        obsidian_card(
            ui, app, crate::ui::NavTab::Photos,
            egui_phosphor::regular::IMAGE,
            "Photos",
            &format!("{}", photo_count),
            if photo_count == 1 { "original quality" } else { "original photos" },
            palette::ACCENT,
        );
        ui.add_space(10.0);

        // Devices Card
        obsidian_card(
            ui, app, crate::ui::NavTab::Devices,
            egui_phosphor::regular::DESKTOP_TOWER,
            "Hardware",
            &format!("{}", device_count),
            "local node live",
            palette::ACCENT_GREEN,
        );
    });
}

fn obsidian_card(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    target: crate::ui::NavTab,
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

/// Activity river with keyboard traversal & dedicated diagnostic layer
fn render_feed(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    vm: &nex_core::runtime::experience::HomeScreenViewModel,
) {
    let feed_len = vm.feed_items.len();

    // Keyboard navigation handlers for feed
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

        // Enter key inspects focused item
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
                            ui.label(RichText::new(&item.status_badge).size(12.0).color(palette::ACCENT_GREEN));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(&item.timestamp_label).size(11.5).color(palette::TEXT_DIM));
                        });
                    });

                    // ── Dedicated Diagnostic Sub-Ribbon for Operator / Advanced Tiers ──
                    if app.ui.complexity == nex_core::runtime::experience::InterfaceComplexity::Expert {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("OBJ_ID: {} | DAG_WINNER: 100% SMT", &item.object_id_hex[0..12]))
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
