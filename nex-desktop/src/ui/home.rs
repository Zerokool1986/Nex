use egui::{Ui, RichText, Frame, Sense};
use nex_core::runtime::experience::HumanExperienceEngine;
use nex_core::runtime::shell::SpaceType;
use crate::app::NexDesktopApp;
use crate::ui::{palette, inspector::SelectedEntity};

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    render_space(ui, app, SpaceType::Personal, "Your NEX", "Local & Private");
}

pub fn render_family(ui: &mut Ui, app: &mut NexDesktopApp) {
    render_space(ui, app, SpaceType::Family, "Family Space", "Shared with family");
}

fn render_space(ui: &mut Ui, app: &mut NexDesktopApp, space: SpaceType, title: &str, subtitle: &str) {
    let vm = HumanExperienceEngine::render_home_screen(
        &app.node, space, app.ui.complexity
    );

    // Hero Greeting Header
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(RichText::new(title).size(28.0).strong().color(palette::TEXT));
            ui.label(RichText::new(subtitle).color(palette::TEXT_DIM).size(14.0));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(format!("{} Inspect Space", egui_phosphor::regular::MAGNIFYING_GLASS)).clicked() {
                app.ui.selected_entity = Some(SelectedEntity::Space(space));
            }
        });
    });
    ui.add_space(12.0);

    // Status & Health Grid
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::SHIELD_CHECK, &vm.sync_status_label)).size(13.0).color(palette::ACCENT_GREEN));
                ui.separator();
                ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::DATABASE, &vm.storage_health_label)).size(13.0).color(palette::TEXT_DIM));
                ui.separator();
                ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::LOCK, &vm.identity_protection_label)).size(13.0).color(palette::TEXT_DIM));
            });
        });

    ui.add_space(16.0);

    if vm.feed_items.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Image::new(egui::include_image!("../../assets/nex_brand_icon.png")).max_height(56.0).max_width(56.0));
            ui.add_space(14.0);
            ui.label(RichText::new("Your Sovereign Sanctuary").size(20.0).strong().color(palette::TEXT));
            ui.add_space(4.0);
            ui.label(RichText::new("Add photos, media, or documents to connect your sovereign mesh")
                .size(13.5).color(palette::TEXT_DIM));
            ui.add_space(16.0);
            if ui.button(format!("{}  Import First File into {}", egui_phosphor::regular::PLUS, title)).clicked() {
                app.ui.active_tab = crate::ui::NavTab::Drive;
            }
        });
    } else {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Recent River — {} items in space", vm.total_items_in_space))
                .strong().color(palette::TEXT).size(14.0));
        });
        ui.add_space(8.0);

        for item in &vm.feed_items {
            let mut obj_id_bytes = [0u8; 32];
            if let Ok(bytes) = hex::decode(&item.object_id_hex) {
                if bytes.len() == 32 {
                    obj_id_bytes.copy_from_slice(&bytes);
                }
            }

            let is_selected = app.ui.selected_entity == Some(SelectedEntity::Object(obj_id_bytes));
            let card_bg = if is_selected { palette::SELECTED } else { palette::PANEL };

            let response = Frame::new()
                .fill(card_bg)
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let icon_glyph = match item.object_type {
                            nex_core::object::types::ObjectType::PhotoMedia => egui_phosphor::regular::IMAGE,
                            nex_core::object::types::ObjectType::DriveInode => egui_phosphor::regular::FILE_TEXT,
                            _ => egui_phosphor::regular::DOTS_THREE,
                        };
                        ui.label(RichText::new(icon_glyph).size(20.0).color(palette::ACCENT));
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&item.title).strong().size(14.0).color(palette::TEXT));
                            ui.label(RichText::new(&item.status_badge)
                                .size(12.0).color(palette::ACCENT_GREEN));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(&item.timestamp_label)
                                .size(11.5).color(palette::TEXT_DIM));
                        });
                    });
                });

            if response.response.interact(Sense::click()).clicked() {
                app.ui.selected_entity = Some(SelectedEntity::Object(obj_id_bytes));
            }
            ui.add_space(6.0);
        }
    }
}
