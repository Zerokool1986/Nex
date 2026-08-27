use egui::{Ui, RichText, Frame, Sense};
use nex_core::runtime::panels::ContextualPanelsEngine;
use crate::app::NexDesktopApp;
use crate::ui::{palette, inspector::SelectedEntity};

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.heading(RichText::new("Devices").size(26.0).strong().color(palette::TEXT));
    ui.label(RichText::new("Your sovereign paired hardware & hosts").color(palette::TEXT_DIM).size(14.0));
    ui.add_space(16.0);

    let actor_id = app.node.identity.actor_id;
    let panel = ContextualPanelsEngine::project_device_panel(&app.node, &actor_id, None, false);

    // This Windows host — always present and live
    let response = Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🖥").size(26.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new("This PC (Windows Host)").strong().size(15.0));
                    ui.label(RichText::new(format!("Node ID: {}", app.actor_id_short()))
                        .size(12.0).color(palette::TEXT_DIM));
                    let sync = app.sync_status();
                    let color = if sync.contains("Online") { palette::ACCENT_GREEN } else { palette::TEXT_DIM };
                    ui.label(RichText::new(format!("Status: {} | Local: {}", sync, panel.is_local_device)).size(12.0).color(color));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("🔍 Click to inspect").size(11.0).color(palette::ACCENT));
                });
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.selected_entity = Some(SelectedEntity::Device(actor_id));
    }

    ui.add_space(16.0);
    ui.label(RichText::new("No other devices paired yet").size(14.0).color(palette::TEXT_DIM));
    ui.label(RichText::new("Pair your Pixel or tablet via QR SAS to synchronize state")
        .size(13.0).color(palette::TEXT_DIM));
}
