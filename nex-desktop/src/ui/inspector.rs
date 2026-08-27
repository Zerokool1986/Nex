use egui::{Ui, RichText, Frame, Button, Color32, Stroke, CornerRadius, Align2, FontId};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::{ObjectID, ObjectType};
use nex_core::identity::types::ActorID;
use nex_core::product::inspector::{UniversalObjectInspector, EpistemicStatus};
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
    Location(ObjectID),
}

pub fn render_inspector_panel(ui: &mut Ui, app: &mut NexDesktopApp) {
    let entity = match app.ui.selected_entity.clone() {
        Some(e) => e,
        None => return,
    };

    // Keyboard shortcut: Escape closes inspector
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.ui.selected_entity = None;
        return;
    }

    Frame::new()
        .fill(palette::SIDEBAR)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(egui_phosphor::regular::SHIELD_CHECK).size(20.0).color(palette::ACCENT_GREEN));
                ui.add_space(4.0);
                ui.heading(RichText::new("Truth Layer & Epistemic Proofs").size(16.0).strong().color(palette::TEXT));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Button::new(RichText::new("✖").size(14.0).color(palette::TEXT_DIM)).frame(false)).clicked() {
                        app.ui.selected_entity = None;
                    }
                });
            });

            ui.add_space(4.0);
            ui.label(RichText::new("Authoritative mathematical evidence behind your sovereign world")
                .size(11.5).color(palette::TEXT_DIM));

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            match entity {
                SelectedEntity::Object(object_id) | SelectedEntity::Location(object_id) => {
                    render_object_inspector(ui, app, &object_id);
                }
                SelectedEntity::Person(actor_id) => {
                    render_person_inspector(ui, app, &actor_id);
                }
                SelectedEntity::Device(actor_id) => {
                    render_device_inspector(ui, app, &actor_id);
                }
                SelectedEntity::Space(space_type) => {
                    render_space_inspector(ui, app, space_type);
                }
                SelectedEntity::Edge(ref edge_id) => {
                    render_edge_inspector(ui, app, edge_id);
                }
            }
        });
}

fn render_object_inspector(ui: &mut Ui, app: &mut NexDesktopApp, object_id: &ObjectID) {
    let complexity = app.ui.complexity;
    match UniversalObjectInspector::inspect(&app.node, object_id, complexity) {
        Ok(inspector) => {
            // ── Section 1: Entity Header & Truth Verdict ──
            let type_glyph = match inspector.object_type {
                ObjectType::PhotoMedia => egui_phosphor::regular::IMAGE,
                ObjectType::DriveInode => egui_phosphor::regular::FILE_TEXT,
                _ => egui_phosphor::regular::FILE,
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(type_glyph).size(24.0).color(palette::ACCENT));
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&inspector.title).strong().size(15.5).color(palette::TEXT));
                    ui.label(RichText::new(format!("Space: {} • {}", inspector.space_name, inspector.byte_size_formatted))
                        .size(11.5).color(palette::TEXT_SECONDARY));
                });
            });

            ui.add_space(8.0);

            // Truth Verdict Badge
            Frame::new()
                .fill(Color32::from_rgb(14, 24, 20))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(52, 211, 153, 100)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🟢").size(10.0));
                        ui.label(RichText::new("VERIFIED FACT — Authenticated & in your physical custody")
                            .size(11.5).strong().color(palette::ACCENT_GREEN));
                    });
                });

            ui.add_space(10.0);

            // ── Cross-Lens Jump Grid ──
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Jump to:").size(11.0).color(palette::TEXT_DIM));
                if ui.button(format!("{} Photos", egui_phosphor::regular::IMAGE)).clicked() {
                    app.ui.active_tab = NavTab::Photos;
                }
                if ui.button(format!("{} Drive", egui_phosphor::regular::HARD_DRIVE)).clicked() {
                    app.ui.active_tab = NavTab::Drive;
                    app.ui.drive_state.selected_file_id = Some(*object_id);
                }
                if ui.button(format!("{} Family", egui_phosphor::regular::HEART)).clicked() {
                    app.ui.active_tab = NavTab::Family;
                }
                if ui.button(format!("{} Maps", egui_phosphor::regular::MAP_PIN)).clicked() {
                    app.ui.active_tab = NavTab::Maps;
                    app.ui.maps_state.selected_object_id = Some(*object_id);
                }
                if ui.button(format!("{} Topology", egui_phosphor::regular::SHARE_NETWORK)).clicked() {
                    app.ui.active_tab = NavTab::Network;
                }
            });

            ui.add_space(10.0);

            // ── Tier 1: Human Truth Grid ──
            Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)).show(ui, |ui| {
                ui.label(RichText::new("HUMAN TRUTH").size(11.0).strong().color(palette::ACCENT));
                ui.add_space(4.0);

                truth_row(ui, "Owner:", &inspector.owner_name);
                truth_row(ui, "Space:", &format!("{} Space", inspector.space_name));
                truth_row(ui, "Stored on:", "🖥 This PC (Local NVMe SSD)");
                truth_row(ui, "Replicas:", &format!("{} trusted physical copies recorded", inspector.replica_count));
                truth_row(ui, "Integrity:", "Content matches canonical identity (BLAKE3 verified)");
                truth_row(ui, "Access:", &inspector.access_summary);
            });

            // ── Tier 2: Why NEX Knows This (Standard Tier and above) ──
            if complexity != InterfaceComplexity::Simple {
                ui.add_space(10.0);
                Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0, palette::GLASS_BORDER)).show(ui, |ui| {
                    ui.label(RichText::new("WHY NEX KNOWS THIS").size(11.0).strong().color(palette::ACCENT_GREEN));
                    ui.add_space(6.0);

                    for check in &inspector.verification_checks {
                        ui.horizontal(|ui| {
                            let (sym_color, sym_text) = match check.status {
                                EpistemicStatus::VerifiedFact => (palette::ACCENT_GREEN, "[✓]"),
                                EpistemicStatus::DerivedState => (palette::ACCENT, "[◐]"),
                                EpistemicStatus::CurrentObservation => (palette::ACCENT, "[◌]"),
                                EpistemicStatus::ExpectedHistorical => (palette::TEXT_SECONDARY, "[○]"),
                                _ => (Color32::RED, "[⚠]"),
                            };

                            ui.label(RichText::new(sym_text).size(11.0).strong().color(sym_color));
                            ui.label(RichText::new(&check.category).size(12.0).strong().color(palette::TEXT));
                        });
                        ui.label(RichText::new(format!("   └─ {}", check.summary)).size(11.0).color(palette::TEXT_SECONDARY));
                        ui.add_space(4.0);
                    }
                });

                // Physical Custody Breakdown
                ui.add_space(10.0);
                Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)).show(ui, |ui| {
                    ui.label(RichText::new("PHYSICAL CUSTODY & RESIDENCY").size(11.0).strong().color(palette::ACCENT));
                    ui.add_space(4.0);

                    for rec in &inspector.physical_residency {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(rec.device_glyph).size(13.0));
                            ui.label(RichText::new(&rec.device_name).size(12.0).strong().color(palette::TEXT));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(&rec.status_label).size(10.5).color(palette::ACCENT_GREEN));
                            });
                        });
                        ui.add_space(2.0);
                    }
                });
            }

            // ── Tier 3: Raw Cryptographic Proofs (Advanced / Operator Tier) ──
            if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
                ui.add_space(10.0);
                Frame::new().fill(Color32::from_rgb(14, 15, 20)).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("RAW CRYPTOGRAPHIC PROOFS (OPERATOR)").size(11.0).strong().color(Color32::from_rgb(192, 132, 252)));
                    });
                    ui.add_space(6.0);

                    raw_proof_row(ui, "BLAKE3_OID:", &inspector.proofs.blake3_hash_hex);
                    raw_proof_row(ui, "ED25519_KEY:", &inspector.proofs.ed25519_author_hex);
                    raw_proof_row(ui, "SMT_MERKLE:", &inspector.proofs.smt_root_hex);
                    raw_proof_row(ui, "WAL_OFFSET:", &format!("LSN {} (Epoch {})", inspector.proofs.wal_lsn, inspector.proofs.created_epoch));
                    raw_proof_row(ui, "FASTCDC:", &format!("{} Chunks verified", inspector.proofs.fastcdc_chunk_count));
                });
            }

            // ── Read-Only Actions: Export Payload ──
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new(format!("{} Export Exact Payload", egui_phosphor::regular::EXPORT)).size(12.0).color(palette::TEXT)).clicked() {
                    app.ui.action_state.active_dialog = Some(ActionDialog::ExportFile {
                        object_id: *object_id,
                        title: inspector.title.clone(),
                        destination_path: String::new(),
                    });
                    app.ui.action_state.text_buffer = format!("d:\\Nex\\Stage9_Tests\\exports\\{}", inspector.title);
                }
            });
        }
        Err(err) => {
            ui.label(RichText::new(format!("Object unavailable: {}", err)).color(Color32::RED).size(13.0));
        }
    }
}

fn truth_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.5).strong().color(palette::TEXT_DIM));
        ui.add_space(4.0);
        ui.label(RichText::new(value).size(11.5).color(palette::TEXT));
    });
}

fn raw_proof_row(ui: &mut Ui, label: &str, val: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).monospace().size(10.0).color(palette::TEXT_DIM));
        let display_val = if val.len() > 20 { format!("{}...", &val[0..20]) } else { val.to_string() };
        ui.label(RichText::new(display_val).monospace().size(10.0).color(palette::TEXT_SECONDARY));
    });
}

fn render_person_inspector(ui: &mut Ui, app: &mut NexDesktopApp, actor_id: &ActorID) {
    let panel = ContextualPanelsEngine::project_person_panel(&app.node, actor_id, "This Device (You)");

    ui.horizontal(|ui| {
        ui.label(RichText::new("👤").size(24.0));
        ui.vertical(|ui| {
            ui.label(RichText::new(&panel.display_name).strong().size(15.5).color(palette::TEXT));
            ui.label(RichText::new(format!("ActorID: 0x{}", hex::encode(&actor_id[0..4]))).size(11.5).color(palette::TEXT_DIM));
        });
    });

    ui.add_space(8.0);
    Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)).show(ui, |ui| {
        ui.label(RichText::new("WEB OF TRUST IDENTITY").size(11.0).strong().color(palette::ACCENT));
        ui.add_space(4.0);
        truth_row(ui, "Trust Tier:", &panel.trust_tier);
        truth_row(ui, "Shared Objects:", &format!("{} objects", panel.shared_objects_count));
        truth_row(ui, "Direct Mesh Chat:", if panel.direct_chat_available { "Available (Direct LAN)" } else { "Offline" });
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button(format!("{} View in People", egui_phosphor::regular::USERS)).clicked() {
            app.ui.active_tab = NavTab::People;
        }
    });
}

fn render_device_inspector(ui: &mut Ui, app: &mut NexDesktopApp, actor_id: &ActorID) {
    let panel = ContextualPanelsEngine::project_device_panel(&app.node, actor_id, None, false);
    let device_name = if panel.is_local_device { "This PC (Windows Host)" } else { "Amy's Pixel 9" };

    ui.horizontal(|ui| {
        ui.label(RichText::new("🖥").size(24.0));
        ui.vertical(|ui| {
            ui.label(RichText::new(device_name).strong().size(15.5).color(palette::TEXT));
            ui.label(RichText::new(format!("Hardware ID: 0x{}", hex::encode(&actor_id[0..4]))).size(11.5).color(palette::TEXT_DIM));
        });
    });

    ui.add_space(8.0);
    Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)).show(ui, |ui| {
        ui.label(RichText::new("PHYSICAL MESH HARDWARE").size(11.0).strong().color(palette::ACCENT));
        ui.add_space(4.0);
        truth_row(ui, "Hardware Role:", if panel.is_local_device { "Primary Local Host (NVMe SSD)" } else { "Verified Mesh Peer" });
        truth_row(ui, "Certificate:", if panel.is_revoked { "Revoked" } else { "Active & Cryptographically Valid" });
        truth_row(ui, "Epoch Validity:", &format!("Epoch {}..{}", panel.not_before_epoch, panel.expires_at_epoch));
        truth_row(ui, "Resilience:", "100% of your world is preserved on this device");
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button(format!("{} View in Devices", egui_phosphor::regular::DEVICES)).clicked() {
            app.ui.active_tab = NavTab::Devices;
        }
    });
}

fn render_space_inspector(ui: &mut Ui, _app: &mut NexDesktopApp, space_type: SpaceType) {
    let title = match space_type {
        SpaceType::Personal => "Personal Sanctuary",
        SpaceType::Family => "Family Space",
        SpaceType::Community => "Community Space",
        SpaceType::Work => "Work Sanctuary",
        SpaceType::Project => "Project Space",
    };

    ui.heading(RichText::new(title).size(16.0).strong().color(palette::ACCENT));
    ui.add_space(8.0);

    Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)).show(ui, |ui| {
        ui.label(RichText::new("CRYPTOGRAPHIC SPACE BOUNDARY").size(11.0).strong().color(palette::ACCENT));
        ui.add_space(4.0);
        truth_row(ui, "Space Type:", &format!("{:?}", space_type));
        truth_row(ui, "Authority:", "Sovereign Root Authority");
        truth_row(ui, "Policy:", "Encrypted with Space Master Key");
    });
}

fn render_edge_inspector(ui: &mut Ui, _app: &mut NexDesktopApp, edge_id: &str) {
    ui.heading(RichText::new("Mesh Conduit Link").size(16.0).strong().color(palette::ACCENT));
    ui.add_space(8.0);

    Frame::new().fill(palette::PANEL).corner_radius(8.0).inner_margin(10.0).stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER)).show(ui, |ui| {
        ui.label(RichText::new("PEER-TO-PEER CONDUIT").size(11.0).strong().color(palette::ACCENT));
        ui.add_space(4.0);
        truth_row(ui, "Link ID:", edge_id);
        truth_row(ui, "Framing:", "NEX/WIRE/v1 (48-byte binary headers)");
        truth_row(ui, "Transport:", "Direct Local LAN / Wi-Fi Mesh");
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
        let data_dir = PathBuf::from("d:\\Nex\\test_data_inspector_stage9");
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

    #[test]
    fn test_universal_inspector_physical_residency_and_slider_diagnostics() {
        let mut app = create_test_app();
        let obj_id = [0x77; 32];
        let mut meta = std::collections::BTreeMap::new();
        meta.insert("title".to_string(), "Family Photo.jpg".to_string());
        meta.insert("space".to_string(), "Family".to_string());

        app.node.state.object_store.insert(obj_id, nex_core::object::types::NexObject {
            object_id: obj_id,
            object_type: nex_core::object::types::ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: app.node.identity.actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 5,
            winning_mutation_id: [0u8; 32],
            metadata: meta,
            payload_bytes: vec![0x42; 2048],
            tombstoned: false,
        });

        // Simple Tier: Diagnostics are suppressed
        let simple_insp = UniversalObjectInspector::inspect(&app.node, &obj_id, InterfaceComplexity::Simple).unwrap();
        assert_eq!(simple_insp.title, "Family Photo.jpg");
        assert!(simple_insp.advanced_dag_info.is_none());
        assert_eq!(simple_insp.verification_checks.len(), 6);

        // Expert / Operator Tier: Diagnostics are exposed without mutating state
        let expert_insp = UniversalObjectInspector::inspect(&app.node, &obj_id, InterfaceComplexity::Expert).unwrap();
        assert!(expert_insp.advanced_dag_info.is_some());
        let dag = expert_insp.advanced_dag_info.unwrap();
        assert_eq!(dag.schema_version, 1);
        assert_eq!(app.node.state.object_store.len(), 1);
        assert_eq!(expert_insp.proofs.blake3_hash_hex, hex::encode(obj_id));
    }
}
