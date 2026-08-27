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

    ui.horizontal(|ui| {
        ui.heading(RichText::new(title).size(26.0).strong().color(palette::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔍 Inspect Space").clicked() {
                app.ui.selected_entity = Some(SelectedEntity::Space(space));
            }
        });
    });
    ui.label(RichText::new(subtitle).color(palette::TEXT_DIM).size(14.0));
    ui.add_space(6.0);

    // Status row
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&vm.sync_status_label).size(13.0).color(palette::ACCENT_GREEN));
                ui.separator();
                ui.label(RichText::new(&vm.storage_health_label).size(13.0).color(palette::TEXT_DIM));
                ui.separator();
                ui.label(RichText::new(&vm.identity_protection_label).size(13.0).color(palette::TEXT_DIM));
            });
        });

    ui.add_space(16.0);

    if vm.feed_items.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Nothing here yet").size(18.0).color(palette::TEXT_DIM));
            ui.add_space(4.0);
            ui.label(RichText::new("Add a photo or file to get started")
                .size(13.0).color(palette::TEXT_DIM));
        });
    } else {
        ui.label(RichText::new(format!("Recent — {} items", vm.total_items_in_space))
            .color(palette::TEXT_DIM).size(13.0));
        ui.add_space(8.0);

        for item in &vm.feed_items {
            let mut obj_id_bytes = [0u8; 32];
            if let Ok(bytes) = hex::decode(&item.object_id_hex) {
                if bytes.len() == 32 {
                    obj_id_bytes.copy_from_slice(&bytes);
                }
            }

            let response = Frame::new()
                .fill(palette::PANEL)
                .corner_radius(6.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let icon = match item.object_type {
                            nex_core::object::types::ObjectType::PhotoMedia => "📷",
                            nex_core::object::types::ObjectType::DriveInode => "📄",
                            _ => "◦",
                        };
                        ui.label(RichText::new(icon).size(18.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&item.title).strong().size(14.0));
                            ui.label(RichText::new(&item.status_badge)
                                .size(12.0).color(palette::TEXT_DIM));
                        });
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
}
