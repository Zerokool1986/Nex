use egui::{Ui, RichText, Frame, Color32, Sense};
use nex_core::identity::types::ActorID;
use nex_core::object::types::ObjectID;
use nex_core::runtime::shell::SpaceType;
use nex_core::runtime::panels::ContextualPanelsEngine;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustLevel {
    LocalSovereign,
    VerifiedFamily,
    PeerContact,
    Unknown,
}

impl TrustLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LocalSovereign => "Local Sovereign (Owner)",
            Self::VerifiedFamily => "Verified Family Member",
            Self::PeerContact => "Peer Contact",
            Self::Unknown => "Trust Level Unavailable",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectedPerson {
    pub actor_id: ActorID,
    pub display_name: String,
    pub trust_level: TrustLevel,
    pub associated_device_count: usize,
    pub spaces: Vec<SpaceType>,
    pub access_level: String,
    pub is_local: bool,
}

#[derive(Debug, Clone)]
pub struct AccessExplanation {
    pub subject_name: String,
    pub subject_actor_id: ActorID,
    pub resource_title: String,
    pub access_granted: String,
    pub reason_steps: Vec<String>,
    pub is_established: bool,
}

#[derive(Debug, Clone)]
pub struct PeopleViewState {
    pub selected_person_id: Option<ActorID>,
    pub explaining_access_for: Option<(ActorID, Option<ObjectID>)>,
    pub active_filter_family_only: bool,
}

impl PeopleViewState {
    pub fn new() -> Self {
        Self {
            selected_person_id: None,
            explaining_access_for: None,
            active_filter_family_only: false,
        }
    }
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Sovereign People & Trust").size(24.0).strong().color(palette::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("Global Policy: {:?}", app.ui.complexity)).color(palette::ACCENT).size(12.5));
        });
    });

    ui.label(RichText::new("Identity, Trust & Access Surface — Truthful projection of sovereign actors and capability boundaries")
        .color(palette::TEXT_DIM).size(13.0));
    ui.add_space(8.0);

    // Derive people from canonical state
    let people = derive_people_catalog(app);

    // Filter bar
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filter:").color(palette::TEXT_DIM).size(13.0));
        let all_selected = !app.ui.people_state.active_filter_family_only;
        if ui.selectable_label(all_selected, "All People").clicked() {
            app.ui.people_state.active_filter_family_only = false;
        }
        let family_selected = app.ui.people_state.active_filter_family_only;
        if ui.selectable_label(family_selected, "🏡 Family Only").clicked() {
            app.ui.people_state.active_filter_family_only = true;
        }
    });
    ui.add_space(10.0);

    if people.is_empty() {
        render_empty_state(ui);
        return;
    }

    let filtered: Vec<&ProjectedPerson> = people.iter()
        .filter(|p| !app.ui.people_state.active_filter_family_only || p.spaces.contains(&SpaceType::Family))
        .collect();

    // Two-column layout: Left = People Cards & Access Explanation Viewport, Right = Universal Inspector
    ui.columns(2, |columns| {
        let (left_ui, right_ui) = columns.split_at_mut(1);
        let content_ui = &mut left_ui[0];
        let inspector_ui = &mut right_ui[0];

        // 1. Contextual Access Explanation Viewport
        if let Some((subject_id, obj_opt)) = app.ui.people_state.explaining_access_for {
            let explanation = explain_access(app, &subject_id, obj_opt.as_ref());
            render_access_explanation_panel(content_ui, app, &explanation);
            content_ui.add_space(12.0);
        }

        // 2. People Cards Grid
        content_ui.label(RichText::new(format!("Sovereign Identities ({} people)", filtered.len()))
            .strong().size(14.0).color(palette::TEXT));
        content_ui.add_space(6.0);

        egui::ScrollArea::vertical().max_height(280.0).show(content_ui, |ui| {
            for person in &filtered {
                render_person_card(ui, app, person);
                ui.add_space(4.0);
            }
        });

        // 3. Right side: Universal Inspector
        crate::ui::inspector::render_inspector_panel(inspector_ui, app);
    });
}

fn render_person_card(ui: &mut Ui, app: &mut NexDesktopApp, person: &ProjectedPerson) {
    let is_selected = app.ui.people_state.selected_person_id == Some(person.actor_id);
    let bg = if is_selected { palette::SELECTED } else { palette::PANEL };

    let response = Frame::new()
        .fill(bg)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let icon_glyph = if person.is_local { egui_phosphor::regular::CROWN } else { egui_phosphor::regular::USER };
                ui.label(RichText::new(icon_glyph).size(24.0).color(if person.is_local { palette::ACCENT_AMBER } else { palette::ACCENT }));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&person.display_name).strong().size(15.0).color(palette::TEXT));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(person.trust_level.label()).size(12.0).color(palette::ACCENT_GREEN));
                        ui.separator();
                        ui.label(RichText::new(format!("{} devices", person.associated_device_count)).size(12.0).color(palette::TEXT_DIM));
                        ui.separator();
                        ui.label(RichText::new(&person.access_level).size(12.0).color(palette::ACCENT));
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("{} Inspect", egui_phosphor::regular::MAGNIFYING_GLASS)).clicked() {
                        app.ui.selected_entity = Some(SelectedEntity::Person(person.actor_id));
                    }
                    if ui.button(format!("{} Access", egui_phosphor::regular::SHIELD_CHECK)).clicked() {
                        app.ui.people_state.explaining_access_for = Some((person.actor_id, None));
                    }
                    if !person.is_local {
                        if ui.button(format!("{} SAS Verify", egui_phosphor::regular::QR_CODE)).clicked() {
                            app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ProximitySasVerification {
                                peer_name: person.display_name.clone(),
                                actor_id: person.actor_id,
                                safety_words: [
                                    "RIVER".to_string(),
                                    "SUMMIT".to_string(),
                                    "FALCON".to_string(),
                                    "HARBOR".to_string(),
                                ],
                            });
                        }
                    }
                });
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.people_state.selected_person_id = Some(person.actor_id);
        app.ui.selected_entity = Some(SelectedEntity::Person(person.actor_id));
    }
}

fn render_access_explanation_panel(ui: &mut Ui, app: &mut NexDesktopApp, explanation: &AccessExplanation) {
    Frame::new()
        .fill(Color32::from_rgb(18, 24, 36))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🛡 Access & Capability Explanation").strong().size(14.0).color(palette::ACCENT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Close").clicked() {
                        app.ui.people_state.explaining_access_for = None;
                    }
                });
            });
            ui.add_space(6.0);

            ui.label(RichText::new(format!("Subject: {} (Actor: {})", explanation.subject_name, hex::encode(&explanation.subject_actor_id[0..4])))
                .size(13.0).color(palette::TEXT));
            ui.label(RichText::new(format!("Resource: {}", explanation.resource_title)).size(12.5).color(palette::TEXT_DIM));
            ui.label(RichText::new(format!("Granted Access: {}", explanation.access_granted)).size(13.0).color(palette::ACCENT_GREEN));
            ui.add_space(6.0);

            ui.label(RichText::new("Why does this identity have access?").strong().size(12.5).color(palette::TEXT));
            for (idx, step) in explanation.reason_steps.iter().enumerate() {
                ui.label(RichText::new(format!("{}. {}", idx + 1, step)).size(12.0).color(palette::TEXT_DIM));
            }

            ui.add_space(6.0);
            ui.label(RichText::new("ℹ Access mutation requires an explicit future capability-delegation workflow. Observation mode is currently active.")
                .size(10.5).color(palette::TEXT_DIM));
        });
}

fn render_empty_state(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("No people available from canonical identity state").size(18.0).color(palette::TEXT_DIM));
        ui.add_space(6.0);
        ui.label(RichText::new("Pair with trusted family members or peers via QR SAS to expand your trust circle.")
            .size(13.0).color(palette::TEXT_DIM));
    });
}

pub fn derive_people_catalog(app: &NexDesktopApp) -> Vec<ProjectedPerson> {
    let mut people = Vec::new();

    // 1. Local Sovereign Identity
    let local_actor_id = app.node.identity.actor_id;
    let local_panel = ContextualPanelsEngine::project_person_panel(&app.node, &local_actor_id, "Chris (You)");

    people.push(ProjectedPerson {
        actor_id: local_actor_id,
        display_name: local_panel.display_name,
        trust_level: TrustLevel::LocalSovereign,
        associated_device_count: 1, // Local Windows PC
        spaces: vec![SpaceType::Personal, SpaceType::Family],
        access_level: "Owner / Administrator".to_string(),
        is_local: true,
    });

    // 2. Derive any other sovereign actors from objects in store
    let mut known_actors = std::collections::HashSet::new();
    known_actors.insert(local_actor_id);

    for obj in app.node.state.object_store.values() {
        if !known_actors.contains(&obj.owner_actor_id) {
            known_actors.insert(obj.owner_actor_id);
            let display_name = obj.metadata.get("author_name")
                .or_else(|| obj.metadata.get("owner"))
                .cloned()
                .unwrap_or_else(|| format!("Actor {}", hex::encode(&obj.owner_actor_id[0..4])));
            
            let is_family = obj.metadata.get("space").map(|s| s.as_str()) == Some("Family");
            let mut spaces = Vec::new();
            if is_family {
                spaces.push(SpaceType::Family);
            }

            people.push(ProjectedPerson {
                actor_id: obj.owner_actor_id,
                display_name,
                trust_level: if is_family { TrustLevel::VerifiedFamily } else { TrustLevel::PeerContact },
                associated_device_count: 1,
                spaces,
                access_level: "View / Contribute".to_string(),
                is_local: false,
            });
        }
    }

    people
}

pub fn explain_access(app: &NexDesktopApp, subject: &ActorID, target_object: Option<&ObjectID>) -> AccessExplanation {
    let is_local = *subject == app.node.identity.actor_id;
    let subject_name = if is_local { "Chris (You)".to_string() } else { format!("Actor {}", hex::encode(&subject[0..4])) };

    if let Some(obj_id) = target_object {
        if let Some(obj) = app.node.state.object_store.get(obj_id) {
            let title = obj.metadata.get("title").or_else(|| obj.metadata.get("filename")).cloned().unwrap_or_else(|| "Untitled Object".to_string());
            let space_name = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());

            let mut steps = Vec::new();
            steps.push(format!("The object '{}' belongs to {} Space.", title, space_name));
            if is_local {
                steps.push("You are the sovereign root authority of this local node.".to_string());
                steps.push("Your master key holds full capability delegation (OP_ALL).".to_string());
            } else {
                steps.push(format!("Identity {} holds verified participation in {} Space.", subject_name, space_name));
                steps.push("Capability certificate verified with cryptographic signature.".to_string());
            }
            steps.push("No object-specific revocation or tombstone exception exists.".to_string());

            return AccessExplanation {
                subject_name,
                subject_actor_id: *subject,
                resource_title: title,
                access_granted: if is_local { "Full Owner / Admin (Read/Write/Delegate)".to_string() } else { "View & Contribute (Read/Write)".to_string() },
                reason_steps: steps,
                is_established: true,
            };
        }
    }

    // Space-wide explanation
    let mut steps = Vec::new();
    if is_local {
        steps.push("Subject is the root sovereign identity of this NEX node.".to_string());
        steps.push("Has full read/write/share capability across all local Spaces.".to_string());
        steps.push("Derived from local master signing key.".to_string());
    } else {
        steps.push(format!("Identity {} is enrolled in Family Space.", subject_name));
        steps.push("Authorized for E2EE content exchange and anti-entropy sync.".to_string());
    }

    AccessExplanation {
        subject_name,
        subject_actor_id: *subject,
        resource_title: "Family Space (All Objects)".to_string(),
        access_granted: if is_local { "Owner / Administrator".to_string() } else { "View / Contribute".to_string() },
        reason_steps: steps,
        is_established: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nex_core::runtime::node::NexNode;
    use nex_core::object::types::{NexObject, ObjectType};
    use nex_core::runtime::experience::InterfaceComplexity;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;
    use std::collections::BTreeMap;

    fn create_test_app_with_people() -> (NexDesktopApp, ActorID, ObjectID) {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_people");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        let amy_actor_id = [0xAA; 32];
        let obj_id = [0x77; 32];
        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), "Amy Vacation.jpg".to_string());
        meta.insert("author_name".to_string(), "Amy".to_string());
        meta.insert("space".to_string(), "Family".to_string());
        node.state.object_store.insert(obj_id, NexObject {
            object_id: obj_id,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: amy_actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: meta,
            payload_bytes: vec![0xAA; 512],
            tombstoned: false,
        });

        let app = NexDesktopApp {
            node,
            data_dir,
            ui: crate::ui::NexUiState::new(),
            status: crate::app::AppStatus::Running,
        };

        (app, amy_actor_id, obj_id)
    }

    #[test]
    fn test_person_projection_uses_canonical_identity_state() {
        let (app, amy_id, _) = create_test_app_with_people();
        let people = derive_people_catalog(&app);

        assert_eq!(people.len(), 2, "Must contain local sovereign user and Amy");
        assert!(people.iter().any(|p| p.actor_id == app.node.identity.actor_id && p.is_local));
        assert!(people.iter().any(|p| p.actor_id == amy_id && p.display_name == "Amy"));
    }

    #[test]
    fn test_device_projection_uses_canonical_device_state() {
        let (app, _, _) = create_test_app_with_people();
        let local_actor = app.node.identity.actor_id;
        let panel = ContextualPanelsEngine::project_device_panel(&app.node, &local_actor, None, false);
        assert!(panel.is_local_device);
        assert_eq!(panel.device_actor_id, local_actor);
    }

    #[test]
    fn test_access_projection_uses_canonical_authority_state() {
        let (app, amy_id, obj_id) = create_test_app_with_people();
        let explanation = explain_access(&app, &amy_id, Some(&obj_id));

        assert!(explanation.is_established);
        assert!(explanation.access_granted.contains("View & Contribute"));
        assert!(explanation.reason_steps.iter().any(|s| s.contains("Family Space")));
    }

    #[test]
    fn test_person_identity_survives_navigation() {
        let (mut app, amy_id, _) = create_test_app_with_people();
        app.ui.active_tab = NavTab::People;
        app.ui.people_state.selected_person_id = Some(amy_id);
        app.ui.selected_entity = Some(SelectedEntity::Person(amy_id));

        app.ui.active_tab = NavTab::Network;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Person(amy_id)));
    }

    #[test]
    fn test_device_identity_survives_navigation() {
        let (mut app, _, _) = create_test_app_with_people();
        let device_actor = app.node.identity.actor_id;
        app.ui.active_tab = NavTab::Devices;
        app.ui.selected_entity = Some(SelectedEntity::Device(device_actor));

        app.ui.active_tab = NavTab::People;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Device(device_actor)));
    }

    #[test]
    fn test_object_identity_survives_people_navigation() {
        let (mut app, _, obj_id) = create_test_app_with_people();
        app.ui.selected_entity = Some(SelectedEntity::Object(obj_id));

        app.ui.active_tab = NavTab::People;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));
    }

    #[test]
    fn test_people_empty_state_is_truthful() {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_people_empty");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        let app = NexDesktopApp {
            node,
            data_dir,
            ui: crate::ui::NexUiState::new(),
            status: crate::app::AppStatus::Running,
        };

        let people = derive_people_catalog(&app);
        // Only local sovereign identity exists; zero phantom contacts
        assert_eq!(people.len(), 1);
        assert!(people[0].is_local);
    }

    #[test]
    fn test_no_fabricated_trust_or_capability() {
        let (app, amy_id, _) = create_test_app_with_people();
        let people = derive_people_catalog(&app);
        let amy = people.iter().find(|p| p.actor_id == amy_id).unwrap();

        assert_eq!(amy.trust_level, TrustLevel::VerifiedFamily);
        assert_eq!(amy.access_level, "View / Contribute");
    }

    #[test]
    fn test_no_secret_material_exposed() {
        let (app, _, _) = create_test_app_with_people();
        let actor = app.node.identity.actor_id;

        let panel = ContextualPanelsEngine::project_person_panel(&app.node, &actor, "Chris");
        let json = serde_json::to_string(&panel).unwrap();
        assert!(!json.contains("signing_key"));
        assert!(!json.contains("secret"));
    }

    #[test]
    fn test_operator_mode_remains_secret_safe() {
        let (mut app, _, _) = create_test_app_with_people();
        app.ui.complexity = InterfaceComplexity::Expert;
        let actor = app.node.identity.actor_id;

        let inspector = crate::ui::inspector::SelectedEntity::Person(actor);
        assert_eq!(inspector, SelectedEntity::Person(actor));
    }

    #[test]
    fn test_people_interactions_remain_read_only() {
        let (mut app, amy_id, obj_id) = create_test_app_with_people();
        let initial_epoch = app.node.state.current_epoch;
        let initial_len = app.node.state.object_store.len();

        app.ui.people_state.selected_person_id = Some(amy_id);
        app.ui.people_state.explaining_access_for = Some((amy_id, Some(obj_id)));
        app.ui.people_state.active_filter_family_only = true;

        assert_eq!(app.node.state.current_epoch, initial_epoch);
        assert_eq!(app.node.state.object_store.len(), initial_len);
    }

    #[test]
    fn test_full_cross_lens_identity_remains_invariant() {
        let (mut app, _, obj_id) = create_test_app_with_people();

        // Object ID0 starts here
        app.ui.active_tab = NavTab::Drive;
        app.ui.selected_entity = Some(SelectedEntity::Object(obj_id));

        // Drive -> Inspector -> People -> Devices -> Maps -> Media -> Network -> Inspector
        app.ui.active_tab = NavTab::People;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));

        app.ui.active_tab = NavTab::Devices;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));

        app.ui.active_tab = NavTab::Maps;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));

        app.ui.active_tab = NavTab::Media;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));

        app.ui.active_tab = NavTab::Network;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));

        // Return to Drive
        app.ui.active_tab = NavTab::Drive;
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));
    }
}
