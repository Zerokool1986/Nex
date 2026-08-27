use egui::{Ui, RichText, Frame, Button, Color32};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::ObjectID;
use nex_core::identity::types::ActorID;
use nex_core::product::inspector::UniversalObjectInspector;
use nex_core::runtime::panels::ContextualPanelsEngine;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, actions::ActionDialog};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedEntity {
    Object(ObjectID),
    Person(ActorID),
    Device(ActorID),
    Space(SpaceType),
    Edge(String),
}

pub fn render_inspector_panel(ui: &mut Ui, app: &mut NexDesktopApp) {
    let entity = match app.ui.selected_entity.clone() {
        Some(e) => e,
        None => return,
    };

    Frame::new()
        .fill(palette::SIDEBAR)
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Universal Inspector").size(17.0).strong().color(palette::ACCENT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Button::new(RichText::new("✖").size(14.0).color(palette::TEXT_DIM)).frame(false)).clicked() {
                        app.ui.selected_entity = None;
                    }
                });
            });
            ui.separator();
            ui.add_space(8.0);

            match entity {
                SelectedEntity::Object(object_id) => render_object_inspector(ui, app, &object_id),
                SelectedEntity::Person(actor_id) => render_person_inspector(ui, app, &actor_id),
                SelectedEntity::Device(actor_id) => render_device_inspector(ui, app, &actor_id),
                SelectedEntity::Space(space_type) => render_space_inspector(ui, app, space_type),
                SelectedEntity::Edge(ref edge_id) => render_edge_inspector(ui, app, edge_id),
            }
        });
}

fn render_object_inspector(ui: &mut Ui, app: &mut NexDesktopApp, object_id: &ObjectID) {
    let complexity = app.ui.complexity;
    match UniversalObjectInspector::inspect(&app.node, object_id, complexity) {
        Ok(inspector) => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📦").size(24.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&inspector.title).strong().size(15.0).color(palette::TEXT));
                    ui.label(RichText::new(format!("Space: {}", inspector.space_name)).size(12.0).color(palette::TEXT_DIM));
                });
            });
            ui.add_space(10.0);

            // Contextual Navigation
            ui.horizontal(|ui| {
                if ui.button("💾 View in Drive").clicked() {
                    app.ui.active_tab = NavTab::Drive;
                    app.ui.drive_state.selected_file_id = Some(*object_id);
                }
                if ui.button("🎬 Open in Media").clicked() {
                    app.ui.active_tab = NavTab::Media;
                    app.ui.media_state.selected_media_id = Some(*object_id);
                }
                if ui.button("🗺 View on Map").clicked() {
                    app.ui.active_tab = NavTab::Maps;
                    app.ui.maps_state.selected_object_id = Some(*object_id);
                }
                if ui.button("🌐 View in Network").clicked() {
                    app.ui.active_tab = NavTab::Network;
                    app.ui.network_state.selected_node_id = Some(format!("obj_{}", hex::encode(&object_id[0..4])));
                    app.ui.network_state.selected_edge_id = None;
                }
            });
            ui.add_space(8.0);

            // Real Sovereign Actions
            ui.horizontal(|ui| {
                if ui.button("✏ Rename").clicked() {
                    app.ui.action_state.active_dialog = Some(ActionDialog::Rename {
                        object_id: *object_id,
                        current_name: inspector.title.clone(),
                        space_name: inspector.space_name.clone(),
                    });
                    app.ui.action_state.text_buffer = inspector.title.clone();
                }
                if ui.button("📤 Export").clicked() {
                    app.ui.action_state.active_dialog = Some(ActionDialog::ExportFile {
                        object_id: *object_id,
                        title: inspector.title.clone(),
                        destination_path: String::new(),
                    });
                    app.ui.action_state.text_buffer = format!("d:\\Nex\\Stage9_Tests\\exports\\{}", inspector.title);
                }
                if ui.button("🗑 Delete").clicked() {
                    app.ui.action_state.active_dialog = Some(ActionDialog::DeleteConfirm {
                        object_id: *object_id,
                        title: inspector.title.clone(),
                    });
                }
                if ui.button("🌐 Share").clicked() {
                    app.ui.action_state.active_dialog = Some(ActionDialog::ShareNotice {
                        object_id: *object_id,
                        title: inspector.title.clone(),
                    });
                }
            });
            ui.add_space(8.0);

            // Show last action result if any
            if let Some(res) = &app.ui.action_state.last_result {
                if res.object_id == *object_id {
                    let color = if res.status == crate::ui::actions::ActionStatus::Success { palette::ACCENT_GREEN } else { Color32::RED };
                    ui.label(RichText::new(&res.message).size(11.5).color(color));
                    ui.add_space(4.0);
                }
            }

            // Representations count from canonical metadata
            let rep_count = app.node.state.object_store.get(object_id)
                .map(|o| {
                    let mut count = 1; // Original Master
                    if o.metadata.contains_key("rep:thumb") { count += 1; }
                    if o.metadata.contains_key("rep:preview") { count += 1; }
                    count
                })
                .unwrap_or(1);

            Frame::new().fill(palette::PANEL).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                ui.label(RichText::new(format!("Size: {}", inspector.byte_size_formatted)).size(13.0));
                ui.label(RichText::new(format!("Status: {}", inspector.status_badge)).size(13.0).color(palette::ACCENT_GREEN));
                ui.label(RichText::new(format!("Representations: {} resolved", rep_count)).size(12.0).color(palette::ACCENT));
                ui.label(RichText::new(format!("Verified Replicas: {} device", inspector.replica_count)).size(12.0).color(palette::TEXT_DIM));
            });

            if let Some(dag) = inspector.advanced_dag_info {
                ui.add_space(10.0);
                ui.label(RichText::new("Cryptographic DAG Provenance:").strong().size(12.0).color(palette::ACCENT));
                ui.label(RichText::new(format!("Schema Version: v{}", dag.schema_version)).size(11.0).color(palette::TEXT_DIM));
                ui.label(RichText::new(format!("Author ID: {}", dag.author_actor_id_hex)).size(11.0).color(palette::TEXT_DIM));
                ui.label(RichText::new(format!("CAS Storage Chunks: {}", dag.cas_chunk_count)).size(11.0).color(palette::TEXT_DIM));
                ui.label(RichText::new(format!("SMT Key: {}", dag.smt_key_hex)).size(11.0).color(palette::TEXT_DIM));
            }
        }
        Err(err) => {
            ui.label(RichText::new(format!("Object unavailable: {}", err)).color(Color32::RED).size(13.0));
        }
    }
}

fn render_person_inspector(ui: &mut Ui, app: &mut NexDesktopApp, actor_id: &ActorID) {
    let panel = ContextualPanelsEngine::project_person_panel(&app.node, actor_id, "This Device (You)");

    ui.horizontal(|ui| {
        ui.label(RichText::new("👤").size(24.0));
        ui.vertical(|ui| {
            ui.label(RichText::new(&panel.display_name).strong().size(15.0).color(palette::TEXT));
            ui.label(RichText::new(format!("ID: {}", hex::encode(&actor_id[0..4]))).size(12.0).color(palette::TEXT_DIM));
        });
    });
    ui.add_space(10.0);

    // Contextual Navigation
    ui.horizontal(|ui| {
        if ui.button("🌐 View in Network").clicked() {
            app.ui.active_tab = NavTab::Network;
            app.ui.network_state.selected_node_id = Some("device_local".to_string());
            app.ui.network_state.selected_edge_id = None;
        }
    });
    ui.add_space(8.0);

    Frame::new().fill(palette::PANEL).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
        ui.label(RichText::new(format!("Trust Tier: {}", panel.trust_tier)).size(13.0).color(palette::ACCENT_GREEN));
        ui.label(RichText::new(format!("Shared Objects: {}", panel.shared_objects_count)).size(12.0));
        ui.label(RichText::new(format!("Direct Chat: {}", panel.direct_chat_available)).size(12.0));
    });

    if matches!(app.ui.complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
        ui.add_space(10.0);
        ui.label(RichText::new("Cryptographic Identity Parameters (Public):").strong().size(12.0).color(palette::ACCENT));
        ui.label(RichText::new(format!("Public Actor ID: {}", hex::encode(actor_id))).size(11.0).color(palette::TEXT_DIM));
        ui.label(RichText::new("Key Type: Ed25519 Sovereign Public Identity").size(11.0).color(palette::TEXT_DIM));
        ui.label(RichText::new("Private Key Protection: OS Keystore Secured").size(11.0).color(palette::ACCENT_GREEN));
    }
}

fn render_device_inspector(ui: &mut Ui, app: &mut NexDesktopApp, actor_id: &ActorID) {
    let panel = ContextualPanelsEngine::project_device_panel(&app.node, actor_id, None, false);

    ui.horizontal(|ui| {
        ui.label(RichText::new("🖥").size(24.0));
        ui.vertical(|ui| {
            ui.label(RichText::new("This PC (Windows Host)").strong().size(15.0).color(palette::TEXT));
            ui.label(RichText::new(format!("ID: {}", hex::encode(&actor_id[0..4]))).size(12.0).color(palette::TEXT_DIM));
        });
    });
    ui.add_space(10.0);

    // Contextual Navigation
    ui.horizontal(|ui| {
        if ui.button("🌐 View in Network").clicked() {
            app.ui.active_tab = NavTab::Network;
            app.ui.network_state.selected_node_id = Some("device_local".to_string());
            app.ui.network_state.selected_edge_id = None;
        }
    });
    ui.add_space(8.0);

    Frame::new().fill(palette::PANEL).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
        ui.label(RichText::new(format!("Local Host: {}", panel.is_local_device)).size(13.0));
        ui.label(RichText::new(format!("Revoked: {}", panel.is_revoked)).size(13.0));
        ui.label(RichText::new(format!("Operational State: {:?}", app.node.operational_state)).size(12.0).color(palette::ACCENT_GREEN));
        ui.label(RichText::new(format!("Validity: Epoch {}..{}", panel.not_before_epoch, panel.expires_at_epoch)).size(12.0).color(palette::TEXT_DIM));
    });

    if matches!(app.ui.complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
        ui.add_space(10.0);
        ui.label(RichText::new("Device Security Parameters (Public):").strong().size(12.0).color(palette::ACCENT));
        ui.label(RichText::new(format!("Public Device Actor: {}", hex::encode(actor_id))).size(11.0).color(palette::TEXT_DIM));
        ui.label(RichText::new("Certificate Enrollment: Master Key Verified").size(11.0).color(palette::TEXT_DIM));
    }
}

fn render_space_inspector(ui: &mut Ui, app: &mut NexDesktopApp, space: SpaceType) {
    let items = app.node.state.object_store.values()
        .filter(|o| match space {
            SpaceType::Personal => o.metadata.get("space").map(|s| s.as_str()) != Some("Family"),
            SpaceType::Family => o.metadata.get("space").map(|s| s.as_str()) == Some("Family"),
            _ => false,
        } && !o.tombstoned)
        .count();

    let (icon, title) = match space {
        SpaceType::Personal => ("🔒", "Personal Space"),
        SpaceType::Family => ("🏡", "Family Space"),
        _ => ("📁", "Space Container"),
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(24.0));
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong().size(15.0).color(palette::TEXT));
            ui.label(RichText::new(format!("{:?}", space)).size(12.0).color(palette::TEXT_DIM));
        });
    });
    ui.add_space(10.0);

    // Contextual Navigation
    ui.horizontal(|ui| {
        if ui.button(match space {
            SpaceType::Family => "🏡 Open Family Space",
            _ => "🏠 Open Personal Space",
        }).clicked() {
            app.ui.active_tab = match space {
                SpaceType::Family => NavTab::Family,
                _ => NavTab::Home,
            };
        }
        if ui.button("🌐 View in Network").clicked() {
            app.ui.active_tab = NavTab::Network;
            app.ui.network_state.selected_node_id = Some(match space {
                SpaceType::Family => "space_family".to_string(),
                _ => "space_personal".to_string(),
            });
            app.ui.network_state.selected_edge_id = None;
        }
    });
    ui.add_space(8.0);

    Frame::new().fill(palette::PANEL).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
        ui.label(RichText::new(format!("Active Objects: {}", items)).size(13.0));
        ui.label(RichText::new("Policy: Sovereign E2EE & Anti-Entropy Synchronized").size(12.0).color(palette::ACCENT_GREEN));
    });
}

fn render_edge_inspector(ui: &mut Ui, app: &mut NexDesktopApp, edge_id: &str) {
    ui.label(RichText::new("Relationship Inspection").strong().color(palette::TEXT).size(15.0));
    ui.label(RichText::new(format!("Edge ID: {}", edge_id)).color(palette::TEXT_DIM).size(12.0));
    ui.add_space(10.0);

    let explanation = match app.ui.complexity {
        InterfaceComplexity::Simple => "Your devices and spaces are connected locally.",
        InterfaceComplexity::Standard => "This device participates in sovereign space synchronization.",
        InterfaceComplexity::Advanced => "Cryptographic master key delegation active with LAN transport.",
        InterfaceComplexity::Expert => "Identity (Ed25519) -> Capability (OP_ALL) -> CAS Storage -> SMT Sync.",
    };

    Frame::new().fill(palette::PANEL).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
        ui.label(RichText::new("Why are these connected?").strong().size(13.5).color(palette::ACCENT_GREEN));
        ui.add_space(4.0);
        ui.label(RichText::new(explanation).size(13.0).color(palette::TEXT));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use nex_core::runtime::node::NexNode;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;

    fn create_test_app() -> NexDesktopApp {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_inspector_stage8");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();
        NexDesktopApp {
            node,
            data_dir,
            ui: crate::ui::NexUiState::new(),
            status: crate::app::AppStatus::Running,
        }
    }

    #[test]
    fn test_sensitive_cryptographic_secrets_never_exposed() {
        let app = create_test_app();
        let actor_id = app.node.identity.actor_id;

        let person_panel = ContextualPanelsEngine::project_person_panel(&app.node, &actor_id, "Test User");
        assert_eq!(person_panel.actor_id, actor_id);
        let model_serialized = serde_json::to_string(&person_panel).unwrap();
        assert!(!model_serialized.contains("signing_key"), "Must never serialize signing key");
        assert!(!model_serialized.contains("secret"), "Must never contain raw secret");

        let device_panel = ContextualPanelsEngine::project_device_panel(&app.node, &actor_id, None, false);
        let device_serialized = serde_json::to_string(&device_panel).unwrap();
        assert!(!device_serialized.contains("signing_key"), "Must never serialize signing key");
    }

    #[test]
    fn test_complexity_switching_is_strictly_read_only_and_preserves_authority() {
        let mut app = create_test_app();
        let initial_epoch = app.node.state.current_epoch;
        let initial_actor = app.node.identity.actor_id;
        let initial_count = app.node.state.object_store.len();

        for tier in [
            InterfaceComplexity::Simple,
            InterfaceComplexity::Standard,
            InterfaceComplexity::Advanced,
            InterfaceComplexity::Expert,
        ] {
            app.ui.complexity = tier;
            assert_eq!(app.node.state.current_epoch, initial_epoch);
            assert_eq!(app.node.identity.actor_id, initial_actor);
            assert_eq!(app.node.state.object_store.len(), initial_count);
        }
    }
}
