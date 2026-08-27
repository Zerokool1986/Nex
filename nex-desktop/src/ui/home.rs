use egui::{Ui, RichText, Frame, Sense, Vec2, Stroke};
use nex_core::runtime::experience::HumanExperienceEngine;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::ObjectType;
use crate::app::NexDesktopApp;
use crate::ui::{palette, inspector::SelectedEntity};

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    render_space(ui, app, SpaceType::Personal, "Personal", "Your sovereign space");
}

pub fn render_family(ui: &mut Ui, app: &mut NexDesktopApp) {
    render_space(ui, app, SpaceType::Family, "Family", "Shared with your family circle");
}

fn render_space(ui: &mut Ui, app: &mut NexDesktopApp, space: SpaceType, title: &str, subtitle: &str) {
    let vm = HumanExperienceEngine::render_home_screen(
        &app.node, space, app.ui.complexity
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // HERO — Large greeting with subtle time-of-day awareness
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let greeting = match space {
        SpaceType::Personal => {
            let hour = 14u32; // In production: chrono::Local::now().hour()
            match hour {
                0..=5 => "Good evening",
                6..=11 => "Good morning",
                12..=17 => "Good afternoon",
                _ => "Good evening",
            }
        }
        SpaceType::Family => "Family",
        _ => "Space",
    };

    ui.label(RichText::new(greeting).size(32.0).color(palette::TEXT));
    ui.add_space(2.0);
    ui.label(RichText::new(subtitle).size(14.0).color(palette::TEXT_SECONDARY));

    ui.add_space(24.0);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATUS BEACON — Compact, honest, non-alarming
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ui.horizontal(|ui| {
        // Sync status
        let sync_text = &vm.sync_status_label;
        let sync_color = if sync_text.contains("Offline") || sync_text.contains("Starting") {
            palette::TEXT_DIM
        } else if sync_text.contains("Local") || sync_text.contains("local") {
            palette::ACCENT_AMBER
        } else {
            palette::ACCENT_GREEN
        };
        ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::SHIELD_CHECK, sync_text))
            .size(12.0).color(sync_color));

        ui.add_space(16.0);
        ui.label(RichText::new("•").size(12.0).color(palette::TEXT_DIM));
        ui.add_space(16.0);

        // Storage
        ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::DATABASE, &vm.storage_health_label))
            .size(12.0).color(palette::TEXT_SECONDARY));

        // Identity — only show in Standard+ mode
        if app.ui.complexity != nex_core::runtime::experience::InterfaceComplexity::Simple {
            ui.add_space(16.0);
            ui.label(RichText::new("•").size(12.0).color(palette::TEXT_DIM));
            ui.add_space(16.0);
            ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::FINGERPRINT, &vm.identity_protection_label))
                .size(12.0).color(palette::TEXT_SECONDARY));
        }
    });

    ui.add_space(28.0);

    if vm.feed_items.is_empty() {
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // EMPTY STATE — Welcoming, warm, actionable
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        render_empty_state(ui, app, title);
    } else {
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // QUICK ACCESS CARDS — Spatial product family cards
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        render_quick_access(ui, app);

        ui.add_space(28.0);

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // ACTIVITY RIVER — Recent items with visual type hierarchy
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        ui.label(RichText::new("Recent Activity").size(14.0).strong().color(palette::TEXT));
        ui.add_space(10.0);

        render_feed(ui, app, &vm);
    }
}

/// Welcoming empty state — not intimidating, clear first action
fn render_empty_state(ui: &mut Ui, app: &mut NexDesktopApp, space_title: &str) {
    let available = ui.available_size();
    let center_y = (available.y * 0.3).max(40.0);

    ui.add_space(center_y);

    ui.vertical_centered(|ui| {
        // Subtle brand mark as spatial anchor
        ui.add(egui::Image::new(egui::include_image!("../../assets/nex_brand_icon.png"))
            .max_height(48.0)
            .max_width(48.0)
            .tint(palette::ACCENT));
        ui.add_space(20.0);

        ui.label(RichText::new("Your space is ready")
            .size(22.0).color(palette::TEXT));
        ui.add_space(6.0);
        ui.label(RichText::new("Add your first file to start building your sovereign library.")
            .size(13.5).color(palette::TEXT_SECONDARY));
        ui.add_space(24.0);

        // Primary action button — distinct from everything else
        let btn = ui.add_sized(
            Vec2::new(200.0, 40.0),
            egui::Button::new(
                RichText::new(format!("{}  Add to {}", egui_phosphor::regular::PLUS, space_title))
                    .size(13.5).color(palette::TEXT)
            )
            .fill(palette::ACCENT)
            .corner_radius(8.0),
        );
        if btn.clicked() {
            app.ui.active_tab = crate::ui::NavTab::Drive;
        }

        ui.add_space(12.0);
        ui.label(RichText::new("or drag files anywhere to import")
            .size(12.0).color(palette::TEXT_DIM));
    });
}

/// Quick access cards — one per product family, showing summary counts
fn render_quick_access(ui: &mut Ui, app: &mut NexDesktopApp) {
    // Count objects by type for this space
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

    let device_count = 1usize; // This PC is always present

    ui.horizontal(|ui| {
        // Card: Files
        quick_card(ui, app, crate::ui::NavTab::Drive,
            egui_phosphor::regular::FOLDER_SIMPLE,
            "Files",
            &format!("{}", file_count + photo_count),
            &format_bytes(total_bytes),
            palette::ACCENT,
        );
        ui.add_space(8.0);

        // Card: Photos
        quick_card(ui, app, crate::ui::NavTab::Photos,
            egui_phosphor::regular::IMAGE,
            "Photos",
            &format!("{}", photo_count),
            if photo_count == 1 { "photo" } else { "photos" },
            palette::ACCENT,
        );
        ui.add_space(8.0);

        // Card: Devices
        quick_card(ui, app, crate::ui::NavTab::Devices,
            egui_phosphor::regular::DESKTOP_TOWER,
            "Devices",
            &format!("{}", device_count),
            "connected",
            palette::ACCENT_GREEN,
        );
    });
}

fn quick_card(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    target: crate::ui::NavTab,
    icon: &str,
    label: &str,
    count: &str,
    detail: &str,
    icon_color: egui::Color32,
) {
    let card_width = ((ui.available_width() - 16.0) / 3.0).max(120.0);

    let response = Frame::new()
        .fill(palette::CARD)
        .corner_radius(10.0)
        .inner_margin(16.0)
        .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(card_width, 80.0));
            ui.vertical(|ui| {
                ui.label(RichText::new(icon).size(22.0).color(icon_color));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(count).size(24.0).strong().color(palette::TEXT));
                    ui.add_space(4.0);
                    ui.label(RichText::new(detail).size(12.0).color(palette::TEXT_SECONDARY));
                });
                ui.label(RichText::new(label).size(12.0).color(palette::TEXT_DIM));
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.active_tab = target;
    }
}

/// Activity feed with visual type differentiation
fn render_feed(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    vm: &nex_core::runtime::experience::HomeScreenViewModel,
) {
    for item in &vm.feed_items {
        let mut obj_id_bytes = [0u8; 32];
        if let Ok(bytes) = hex::decode(&item.object_id_hex) {
            if bytes.len() == 32 {
                obj_id_bytes.copy_from_slice(&bytes);
            }
        }

        let is_selected = app.ui.selected_entity == Some(SelectedEntity::Object(obj_id_bytes));
        let card_bg = if is_selected { palette::SELECTED } else { palette::CARD };
        let border = if is_selected {
            Stroke::new(1.0, palette::ACCENT)
        } else {
            Stroke::new(1.0, palette::BORDER_SUBTLE)
        };

        let (icon, icon_color) = match item.object_type {
            ObjectType::PhotoMedia => (egui_phosphor::regular::IMAGE, palette::ACCENT),
            ObjectType::DriveInode => (egui_phosphor::regular::FILE_TEXT, palette::TEXT_SECONDARY),
            _ => (egui_phosphor::regular::CUBE, palette::TEXT_DIM),
        };

        let response = Frame::new()
            .fill(card_bg)
            .corner_radius(8.0)
            .inner_margin(egui::Margin { left: 14, right: 14, top: 10, bottom: 10 })
            .stroke(border)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Type icon
                    ui.label(RichText::new(icon).size(18.0).color(icon_color));
                    ui.add_space(8.0);

                    // Title + status
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&item.title).size(13.5).color(palette::TEXT));
                        ui.label(RichText::new(&item.status_badge)
                            .size(11.5).color(palette::TEXT_SECONDARY));
                    });

                    // Timestamp right-aligned
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(&item.timestamp_label)
                            .size(11.0).color(palette::TEXT_DIM));
                    });
                });
            });

        if response.response.interact(Sense::click()).clicked() {
            app.ui.selected_entity = Some(SelectedEntity::Object(obj_id_bytes));
        }
        ui.add_space(4.0);
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
