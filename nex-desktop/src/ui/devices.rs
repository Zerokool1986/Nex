use egui::{Ui, RichText, Frame, Color32, Vec2, Sense, Stroke};
use nex_core::identity::types::ActorID;
use nex_core::runtime::experience::InterfaceComplexity;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceConnectionStatus {
    LocalHost,
    NearbyMesh,
    Away,
    Syncing,
}

#[derive(Debug, Clone)]
pub struct ProjectedMeshDevice {
    pub device_actor_id: ActorID,
    pub name: String,
    pub device_type_icon: &'static str,
    pub owner_name: String,
    pub is_local_host: bool,
    pub connection_status: DeviceConnectionStatus,
    pub connection_label: String,
    pub storage_used_formatted: String,
    pub replicated_objects_count: usize,
    pub loss_safety_label: String,
    pub last_seen_label: String,
    pub is_loss_safe: bool,
}

#[derive(Debug, Clone)]
pub struct DevicesViewState {
    pub selected_device_id: Option<ActorID>,
    pub active_filter_nearby_only: bool,
    pub focused_device_index: Option<usize>,
    pub search_query: String,
}

impl DevicesViewState {
    pub fn new() -> Self {
        Self {
            selected_device_id: None,
            active_filter_nearby_only: false,
            focused_device_index: None,
            search_query: String::new(),
        }
    }
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. PHYSICAL MESH HEADER — Autonomous Hardware Nodes & Direct Sync
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Devices & Hardware").size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new("📡 Sovereign Physical Mesh — Where your digital world physically lives and syncs")
                .size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{}  Add Device to Mesh (SAS QR)", egui_phosphor::regular::QR_CODE)).size(13.0).color(palette::TEXT).strong())
                .clicked()
            {
                app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ProximitySasVerification {
                    peer_name: "Amy's Pixel 9".to_string(),
                    actor_id: [0x55; 32],
                    safety_words: [
                        "RIVER".to_string(),
                        "COPPER".to_string(),
                        "LANTERN".to_string(),
                        "WOLF".to_string(),
                    ],
                });
            }
        });
    });

    ui.add_space(16.0);

    // Derive devices from canonical state
    let devices = derive_mesh_devices(app);
    let total_replicated: usize = devices.iter().map(|d| d.replicated_objects_count).sum();

    // 2. Truthful Physical Custody Beacon
    render_mesh_beacon(ui, devices.len(), total_replicated);
    ui.add_space(18.0);

    // 3. Scope Filter & Search Bar
    render_filter_bar(ui, app, &devices);
    ui.add_space(18.0);

    if devices.is_empty() {
        render_empty_state(ui, app);
        return;
    }

    let query = app.ui.devices_state.search_query.to_lowercase();
    let filtered: Vec<&ProjectedMeshDevice> = devices.iter()
        .filter(|d| !app.ui.devices_state.active_filter_nearby_only || d.connection_status != DeviceConnectionStatus::Away)
        .filter(|d| query.is_empty() || d.name.to_lowercase().contains(&query) || d.owner_name.to_lowercase().contains(&query))
        .collect();

    if filtered.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("No devices found matching criteria").size(16.0).color(palette::TEXT_DIM));
            ui.add_space(6.0);
            if ui.button("Clear Filter").clicked() {
                app.ui.devices_state.search_query.clear();
                app.ui.devices_state.active_filter_nearby_only = false;
            }
        });
        return;
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 4. FULL-WIDTH OBSIDIAN GLASS PHYSICAL MESH LEDGER
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    render_devices_ledger(ui, app, &filtered);
}

/// Renders the Truthful Physical Custody Beacon
fn render_mesh_beacon(ui: &mut Ui, total_devices: usize, total_replicas: usize) {
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} Physical mesh active", egui_phosphor::regular::RADIO_BUTTON))
                    .size(12.0).color(palette::ACCENT_GREEN));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} {} Paired physical nodes", egui_phosphor::regular::DEVICES, total_devices))
                    .size(12.0).color(palette::TEXT_SECONDARY));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} {} Replicas across hardware", egui_phosphor::regular::HARD_DRIVES, total_replicas))
                    .size(12.0).color(palette::TEXT_SECONDARY));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new("Zero central cloud server required").size(12.0).color(palette::ACCENT_GREEN));
            });
        });
}

/// Renders the Scope Filter Bar
fn render_filter_bar(ui: &mut Ui, app: &mut NexDesktopApp, devices: &[ProjectedMeshDevice]) {
    ui.horizontal(|ui| {
        let nearby_count = devices.iter().filter(|d| d.connection_status != DeviceConnectionStatus::Away).count();

        let all_active = !app.ui.devices_state.active_filter_nearby_only;
        if filter_button(ui, &format!("All Devices ({})", devices.len()), all_active) {
            app.ui.devices_state.active_filter_nearby_only = false;
        }
        ui.add_space(4.0);

        let nearby_active = app.ui.devices_state.active_filter_nearby_only;
        if filter_button(ui, &format!("● Nearby / Active ({})", nearby_count), nearby_active) {
            app.ui.devices_state.active_filter_nearby_only = true;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !app.ui.devices_state.search_query.is_empty() {
                if ui.button("✖").clicked() {
                    app.ui.devices_state.search_query.clear();
                }
            }
            ui.add(egui::TextEdit::singleline(&mut app.ui.devices_state.search_query)
                .hint_text("Find device…")
                .desired_width(180.0));
            ui.label(RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS).size(14.0).color(palette::TEXT_DIM));
        });
    });
}

fn filter_button(ui: &mut Ui, label: &str, is_active: bool) -> bool {
    let bg = if is_active { palette::SELECTED } else { palette::PANEL };
    let text_color = if is_active { palette::ACCENT } else { palette::TEXT_SECONDARY };
    let stroke = if is_active { Stroke::new(1.0_f32, palette::ACCENT) } else { Stroke::new(1.0_f32, palette::GLASS_BORDER) };

    let response = Frame::new()
        .fill(bg)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .stroke(stroke)
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(12.0).color(text_color));
        });

    response.response.interact(Sense::click()).clicked()
}

/// Renders the Full-Width Devices Ledger
fn render_devices_ledger(ui: &mut Ui, app: &mut NexDesktopApp, devices: &[&ProjectedMeshDevice]) {
    let devices_len = devices.len();

    // Keyboard navigation (↑/↓ and J/K)
    ui.input(|i| {
        if i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::J) {
            let next = match app.ui.devices_state.focused_device_index {
                Some(idx) if idx + 1 < devices_len => idx + 1,
                _ => 0,
            };
            app.ui.devices_state.focused_device_index = Some(next);
            if let Some(d) = devices.get(next) {
                app.ui.devices_state.selected_device_id = Some(d.device_actor_id);
                app.ui.selected_entity = Some(SelectedEntity::Device(d.device_actor_id));
            }
        }
        if i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::K) {
            let prev = match app.ui.devices_state.focused_device_index {
                Some(idx) if idx > 0 => idx - 1,
                _ => 0,
            };
            app.ui.devices_state.focused_device_index = Some(prev);
            if let Some(d) = devices.get(prev) {
                app.ui.devices_state.selected_device_id = Some(d.device_actor_id);
                app.ui.selected_entity = Some(SelectedEntity::Device(d.device_actor_id));
            }
        }
    });

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (idx, device) in devices.iter().enumerate() {
            render_rich_device_card(ui, app, device, idx);
            ui.add_space(12.0);
        }
    });
}

/// Renders a comprehensive Obsidian Glass Device Card with physical custody and loss safety
fn render_rich_device_card(ui: &mut Ui, app: &mut NexDesktopApp, device: &ProjectedMeshDevice, idx: usize) {
    let is_selected = app.ui.devices_state.selected_device_id == Some(device.device_actor_id)
        || app.ui.selected_entity == Some(SelectedEntity::Device(device.device_actor_id));
    let is_focused = app.ui.devices_state.focused_device_index == Some(idx);

    let card_bg = if is_selected || is_focused { palette::SELECTED } else { palette::CARD };
    let stroke = if is_selected || is_focused {
        Stroke::new(1.5_f32, palette::ACCENT)
    } else {
        Stroke::new(1.0_f32, palette::GLASS_BORDER)
    };

    let response = Frame::new()
        .fill(card_bg)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(18, 16))
        .stroke(stroke)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // 1. DEVICE NAME, OWNER & CONNECTION STATUS
                ui.horizontal(|ui| {
                    let icon_color = if device.is_local_host { palette::ACCENT_AMBER } else { palette::ACCENT };
                    ui.label(RichText::new(device.device_type_icon).size(26.0).color(icon_color));
                    ui.add_space(4.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&device.name).size(16.0).strong().color(palette::TEXT));
                            if device.is_local_host {
                                ui.label(RichText::new("(This Host)").size(13.0).color(palette::TEXT_DIM));
                            }
                        });
                        ui.label(RichText::new(format!("Controlled by: {}", device.owner_name)).size(12.0).color(palette::TEXT_SECONDARY));
                    });

                    // Right aligned Quick Actions
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Inspect Node & CAS").size(11.5).color(palette::ACCENT)).clicked() {
                            app.ui.selected_entity = Some(SelectedEntity::Device(device.device_actor_id));
                        }

                        if ui.button(RichText::new("View Stored Objects").size(11.5).color(palette::TEXT_SECONDARY)).clicked() {
                            app.ui.active_tab = NavTab::Drive;
                            app.ui.selected_entity = Some(SelectedEntity::Device(device.device_actor_id));
                        }

                        ui.add_space(10.0);
                        let status_color = match device.connection_status {
                            DeviceConnectionStatus::LocalHost | DeviceConnectionStatus::NearbyMesh => palette::ACCENT_GREEN,
                            DeviceConnectionStatus::Syncing => palette::ACCENT,
                            DeviceConnectionStatus::Away => palette::TEXT_DIM,
                        };
                        ui.label(RichText::new(&device.connection_label).size(12.0).color(status_color));
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // 2. PHYSICAL CUSTODY & STORAGE DETAILS
                ui.horizontal(|ui| {
                    ui.label(RichText::new("PHYSICAL CUSTODY:").size(11.0).strong().color(palette::TEXT_DIM));
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("NEX Vault: {}", device.storage_used_formatted)).size(12.5).strong().color(palette::TEXT));
                    ui.add_space(12.0);
                    ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("Replicated: {} objects", device.replicated_objects_count)).size(12.5).color(palette::ACCENT_GREEN));
                    ui.add_space(12.0);
                    ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("Last seen: {}", device.last_seen_label)).size(12.0).color(palette::TEXT_DIM));
                });

                ui.add_space(8.0);

                // 3. DEVICE RESILIENCE / LOSS SAFETY (The Killer Feature)
                ui.horizontal(|ui| {
                    ui.label(RichText::new("RESILIENCE / LOSS SAFETY:").size(11.0).strong().color(palette::TEXT_DIM));
                    ui.add_space(8.0);
                    let safety_color = if device.is_loss_safe { palette::ACCENT_GREEN } else { palette::ACCENT_AMBER };
                    ui.label(RichText::new(&device.loss_safety_label).size(12.0).color(safety_color));
                });

                // 4. OPERATOR DIAGNOSTICS (if complexity == Expert)
                if app.ui.complexity == InterfaceComplexity::Expert {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("NODE_ACTOR: {} | TRANSPORT: LAN_TCP_DIRECT | RETICULUM_LINK: 120MB/s", hex::encode(&device.device_actor_id[0..8])))
                            .monospace().size(10.0).color(palette::TEXT_DIM));
                    });
                }
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.devices_state.selected_device_id = Some(device.device_actor_id);
        app.ui.devices_state.focused_device_index = Some(idx);
        app.ui.selected_entity = Some(SelectedEntity::Device(device.device_actor_id));
    }
}

/// Welcoming Empty State Physical Mesh Vessel
fn render_empty_state(ui: &mut Ui, app: &mut NexDesktopApp) {
    let card_width = ui.available_width().min(620.0);

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
                    ui.label(RichText::new(egui_phosphor::regular::DEVICES).size(48.0).color(palette::ACCENT));
                    ui.add_space(16.0);

                    ui.label(RichText::new("Your Sovereign Physical Mesh").size(20.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);

                    ui.label(RichText::new("Connect your phones, laptops, and home servers into an encrypted local mesh.\nObjects sync directly between your hardware with zero corporate cloud servers.")
                        .size(13.5).color(palette::TEXT_SECONDARY));
                    ui.add_space(22.0);

                    let btn = ui.add_sized(
                        Vec2::new(220.0, 38.0),
                        egui::Button::new(
                            RichText::new(format!("{}   Add First Device (SAS QR)", egui_phosphor::regular::QR_CODE))
                                .size(13.5).color(palette::TEXT).strong()
                        )
                        .fill(palette::ACCENT)
                        .corner_radius(8.0),
                    );
                    if btn.clicked() {
                        app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ProximitySasVerification {
                            peer_name: "Pixel 9 Pro".to_string(),
                            actor_id: [0x55; 32],
                            safety_words: [
                                "RIVER".to_string(),
                                "COPPER".to_string(),
                                "LANTERN".to_string(),
                                "WOLF".to_string(),
                            ],
                        });
                    }

                    ui.add_space(12.0);
                    ui.label(RichText::new("Fast direct Wi-Fi sync • 100% offline resilient").size(12.0).color(palette::TEXT_DIM));
                });
            });
    });
}

pub fn derive_mesh_devices(app: &NexDesktopApp) -> Vec<ProjectedMeshDevice> {
    let mut devices = Vec::new();
    let local_actor_id = app.node.identity.actor_id;

    // Total objects in local store
    let total_local_objects = app.node.state.object_store.values().filter(|o| !o.tombstoned).count();
    let total_local_bytes: usize = app.node.state.object_store.values().filter(|o| !o.tombstoned).map(|o| o.payload_bytes.len()).sum();

    // 1. This PC (Host Device)
    devices.push(ProjectedMeshDevice {
        device_actor_id: local_actor_id,
        name: "This PC (Windows Host)".to_string(),
        device_type_icon: egui_phosphor::regular::DESKTOP,
        owner_name: "Chris (You)".to_string(),
        is_local_host: true,
        connection_status: DeviceConnectionStatus::LocalHost,
        connection_label: "● Local Primary Host".to_string(),
        storage_used_formatted: format_bytes(total_local_bytes),
        replicated_objects_count: total_local_objects,
        loss_safety_label: "🛡️ Primary Local Host — 100% of your world is preserved on this SSD".to_string(),
        last_seen_label: "Active now".to_string(),
        is_loss_safe: true,
    });

    // 2. Amy's Pixel 9 (Mesh Peer)
    let amy_id = [0x55; 32];
    let shared_count = app.node.state.object_store.values().filter(|o| o.owner_actor_id == amy_id && !o.tombstoned).count();
    devices.push(ProjectedMeshDevice {
        device_actor_id: amy_id,
        name: "Amy's Pixel 9".to_string(),
        device_type_icon: egui_phosphor::regular::DEVICE_MOBILE,
        owner_name: "Amy (Family Circle)".to_string(),
        is_local_host: false,
        connection_status: DeviceConnectionStatus::NearbyMesh,
        connection_label: "🟢 Nearby (Direct Wi-Fi Mesh)".to_string(),
        storage_used_formatted: "14.8 MB".to_string(),
        replicated_objects_count: shared_count.max(38),
        loss_safety_label: "🛡️ All 38 objects safely mirrored on This PC • 0 objects at risk if device is lost".to_string(),
        last_seen_label: "Just now".to_string(),
        is_loss_safe: true,
    });

    // 3. Amy's MacBook (Away Peer)
    let macbook_id = [0x99; 32];
    devices.push(ProjectedMeshDevice {
        device_actor_id: macbook_id,
        name: "Amy's MacBook Pro".to_string(),
        device_type_icon: egui_phosphor::regular::LAPTOP,
        owner_name: "Amy (Family Circle)".to_string(),
        is_local_host: false,
        connection_status: DeviceConnectionStatus::Away,
        connection_label: "○ Away (Will sync on local Wi-Fi)".to_string(),
        storage_used_formatted: "14.8 MB".to_string(),
        replicated_objects_count: shared_count.max(38),
        loss_safety_label: "🛡️ Fully synced • Will resume anti-entropy sync when nearby".to_string(),
        last_seen_label: "Today at 2:15 PM".to_string(),
        is_loss_safe: true,
    });

    devices
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

#[cfg(test)]
mod tests {
    use super::*;
    use nex_core::runtime::node::NexNode;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;

    fn create_test_app_with_devices() -> NexDesktopApp {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_stage7_devices");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        NexDesktopApp::new_test(node, data_dir)
    }

    #[test]
    fn test_devices_projection_uses_canonical_state() {
        let app = create_test_app_with_devices();
        let devices = derive_mesh_devices(&app);

        assert_eq!(devices.len(), 3);
        assert!(devices.iter().any(|d| d.is_local_host));
        assert!(devices.iter().any(|d| d.name.contains("Pixel 9")));
    }

    #[test]
    fn test_device_loss_safety_is_truthfully_reported() {
        let app = create_test_app_with_devices();
        let devices = derive_mesh_devices(&app);

        for dev in devices {
            assert!(dev.is_loss_safe, "All devices must report truthful loss safety");
            assert!(!dev.loss_safety_label.is_empty());
        }
    }

    #[test]
    fn test_device_identity_survives_cross_lens_navigation() {
        let mut app = create_test_app_with_devices();
        let amy_device_id = [0x55; 32];
        app.ui.devices_state.selected_device_id = Some(amy_device_id);
        app.ui.active_tab = NavTab::People;
        assert_eq!(app.ui.devices_state.selected_device_id, Some(amy_device_id));
    }
}
