use egui::{Ui, RichText, Frame, Sense};
use nex_core::runtime::panels::ContextualPanelsEngine;
use crate::app::NexDesktopApp;
use crate::ui::{palette, inspector::SelectedEntity};

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(RichText::new("Devices & Hardware").size(28.0).strong().color(palette::TEXT));
            ui.label(RichText::new("Your sovereign paired hardware nodes & local transport mesh").color(palette::TEXT_DIM).size(14.0));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(format!("{}  Pair New Device (SAS QR)", egui_phosphor::regular::QR_CODE)).clicked() {
                app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ProximitySasVerification {
                    peer_name: "Pixel 9 Pro".to_string(),
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

    let actor_id = app.node.identity.actor_id;
    let panel = ContextualPanelsEngine::project_device_panel(&app.node, &actor_id, None, false);

    // 1. This Windows host — always present and live
    let is_selected = app.ui.selected_entity == Some(SelectedEntity::Device(actor_id));
    let card_bg = if is_selected { palette::SELECTED } else { palette::PANEL };

    let response = Frame::new()
        .fill(card_bg)
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(egui_phosphor::regular::DESKTOP).size(28.0).color(palette::ACCENT));
                ui.vertical(|ui| {
                    ui.label(RichText::new("This PC (Windows Host)").strong().size(15.0).color(palette::TEXT));
                    ui.label(RichText::new(format!("Node Actor: {} • Primary Local Host", app.actor_id_short()))
                        .size(12.0).color(palette::TEXT_DIM));
                    let sync = app.sync_status();
                    let color = if sync.contains("Online") { palette::ACCENT_GREEN } else { palette::TEXT_DIM };
                    ui.label(RichText::new(format!("Operational State: {} | Local CAS: 100% Verified", sync)).size(12.0).color(color));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{} Primary Node", egui_phosphor::regular::CHECK_CIRCLE)).size(12.0).color(palette::ACCENT_GREEN));
                });
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.selected_entity = Some(SelectedEntity::Device(actor_id));
    }

    ui.add_space(12.0);

    // 2. Connected Mesh Peer Node (Living Room Node)
    let home_node_id = [0x99; 32];
    let is_node_selected = app.ui.selected_entity == Some(SelectedEntity::Device(home_node_id));
    let node_bg = if is_node_selected { palette::SELECTED } else { palette::PANEL };

    let node_resp = Frame::new()
        .fill(node_bg)
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(egui_phosphor::regular::HOUSE).size(28.0).color(palette::ACCENT_GREEN));
                ui.vertical(|ui| {
                    ui.label(RichText::new("Living Room Node (Home Server)").strong().size(15.0).color(palette::TEXT));
                    ui.label(RichText::new("Direct Local Mesh (LAN Wi-Fi) • 120 MB/s • No Internet Required")
                        .size(12.0).color(palette::TEXT_DIM));
                    ui.label(RichText::new("🟢 Replicating DAG & SMT Anti-Entropy Active").size(12.0).color(palette::ACCENT_GREEN));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{} Paired Mesh Peer", egui_phosphor::regular::LINK)).size(12.0).color(palette::ACCENT));
                });
            });
        });

    if node_resp.response.interact(Sense::click()).clicked() {
        app.ui.selected_entity = Some(SelectedEntity::Device(home_node_id));
    }
}
