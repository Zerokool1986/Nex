use egui::{Ui, RichText, Frame};
use nex_core::runtime::production::NodeOperationalState;
use crate::app::NexDesktopApp;
use crate::ui::palette;

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.heading(RichText::new("Settings").size(26.0).strong().color(palette::TEXT));
    ui.add_space(16.0);

    section(ui, "Identity", |ui| {
        kv(ui, "Node ID", &hex::encode(&app.node.identity.actor_id));
        kv(ui, "Key type", "Ed25519 (software key)");
        kv(ui, "Schema version", &format!("v{}", app.node.schema_version));
    });

    ui.add_space(12.0);

    section(ui, "Storage", |ui| {
        kv(ui, "Data directory", &app.data_dir.display().to_string());
        kv(ui, "Objects stored", &format!("{}", app.object_count()));
        let op = match app.node.operational_state {
            NodeOperationalState::Running => "Running",
            NodeOperationalState::Degraded => "Degraded",
            NodeOperationalState::Uninitialized => "Uninitialized",
            NodeOperationalState::ReplayingWal => "Replaying WAL",
            NodeOperationalState::Stopped => "Stopped",
        };
        kv(ui, "Node state", op);
    });

    ui.add_space(12.0);

    section(ui, "About", |ui| {
        kv(ui, "NEX version", "0.1.0-alpha");
        kv(ui, "Core", "nex-core 0.1.0");
        kv(ui, "UI framework", "egui 0.31 / eframe 0.31");
        kv(ui, "Rust", "stable-x86_64-pc-windows-msvc");
    });
}

fn section(ui: &mut Ui, title: &str, contents: impl FnOnce(&mut Ui)) {
    ui.label(RichText::new(title).strong().size(15.0).color(palette::ACCENT));
    ui.add_space(4.0);
    Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(12.0).show(ui, contents);
}

fn kv(ui: &mut Ui, key: &str, val: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).size(13.0).color(palette::TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(val).size(13.0).color(palette::TEXT)
                .monospace());
        });
    });
}
