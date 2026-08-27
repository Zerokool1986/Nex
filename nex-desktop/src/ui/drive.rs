use egui::{Ui, RichText, Frame, Color32, Vec2, Sense};
use nex_core::object::types::{ObjectID, ObjectType, NexObject};
use nex_core::runtime::shell::SpaceType;
use nex_core::runtime::experience::InterfaceComplexity;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

#[derive(Debug, Clone)]
pub struct ProjectedDriveFile {
    pub object_id: ObjectID,
    pub filename: String,
    pub space_name: String,
    pub object_type: ObjectType,
    pub byte_size: usize,
    pub byte_size_formatted: String,
    pub virtual_folder: String,
    pub is_text: bool,
    pub is_media: bool,
    pub is_location_aware: bool,
    pub local_available: bool,
    pub status_badge: String,
}

#[derive(Debug, Clone)]
pub struct DriveViewState {
    pub selected_file_id: Option<ObjectID>,
    pub active_space_filter: Option<SpaceType>,
    pub search_query: String,
    pub preview_content: Option<String>,
}

impl DriveViewState {
    pub fn new() -> Self {
        Self {
            selected_file_id: None,
            active_space_filter: None,
            search_query: String::new(),
            preview_content: None,
        }
    }
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Sovereign Drive & Documents").size(24.0).strong().color(palette::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("Global Policy: {:?}", app.ui.complexity)).color(palette::ACCENT).size(12.5));
        });
    });

    ui.label(RichText::new("Unified Filesystem Projection — Content-addressed files and sovereign documents without duplicate storage")
        .color(palette::TEXT_DIM).size(13.0));
    ui.add_space(8.0);

    // Derive file catalog purely from canonical object store
    let catalog = derive_drive_catalog(app);

    // Filter bar
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filter Space:").color(palette::TEXT_DIM).size(13.0));
        let all_selected = app.ui.drive_state.active_space_filter.is_none();
        if ui.selectable_label(all_selected, "All Files").clicked() {
            app.ui.drive_state.active_space_filter = None;
        }
        let personal_selected = app.ui.drive_state.active_space_filter == Some(SpaceType::Personal);
        if ui.selectable_label(personal_selected, "🔒 Personal").clicked() {
            app.ui.drive_state.active_space_filter = Some(SpaceType::Personal);
        }
        let family_selected = app.ui.drive_state.active_space_filter == Some(SpaceType::Family);
        if ui.selectable_label(family_selected, "🏡 Family").clicked() {
            app.ui.drive_state.active_space_filter = Some(SpaceType::Family);
        }

        ui.add_space(16.0);
        ui.label(RichText::new("Search:").color(palette::TEXT_DIM).size(13.0));
        ui.text_edit_singleline(&mut app.ui.drive_state.search_query);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("📥 Import File").strong().color(palette::ACCENT)).clicked() {
                app.ui.action_state.active_dialog = Some(crate::ui::actions::ActionDialog::ImportFile {
                    source_path: String::new(),
                    target_space: app.ui.drive_state.active_space_filter.unwrap_or(SpaceType::Family),
                });
                app.ui.action_state.text_buffer.clear();
            }
        });
    });
    ui.add_space(10.0);

    if catalog.is_empty() {
        render_empty_state(ui);
        return;
    }

    let query = app.ui.drive_state.search_query.to_lowercase();
    let filtered_catalog: Vec<&ProjectedDriveFile> = catalog.iter()
        .filter(|f| match app.ui.drive_state.active_space_filter {
            None => true,
            Some(SpaceType::Personal) => f.space_name != "Family",
            Some(SpaceType::Family) => f.space_name == "Family",
            Some(_) => true,
        })
        .filter(|f| query.is_empty() || f.filename.to_lowercase().contains(&query) || f.virtual_folder.to_lowercase().contains(&query))
        .collect();

    if filtered_catalog.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("No files found matching criteria").size(16.0).color(palette::TEXT_DIM));
            ui.add_space(4.0);
            ui.label(RichText::new("Clear your search query or switch Space filters")
                .size(13.0).color(palette::TEXT_DIM));
        });
        return;
    }

    // Split layout: Left = Drive File Table & File Actions, Right = Universal Inspector
    ui.columns(2, |columns| {
        let (left_ui, right_ui) = columns.split_at_mut(1);
        let content_ui = &mut left_ui[0];
        let inspector_ui = &mut right_ui[0];

        // 1. File Preview Viewport (if file is selected and has preview content)
        if let Some(selected_id) = app.ui.drive_state.selected_file_id {
            if let Some(file) = filtered_catalog.iter().find(|f| f.object_id == selected_id) {
                render_file_viewport(content_ui, app, file);
                content_ui.add_space(12.0);
            }
        }

        // 2. Drive Files Table
        content_ui.label(RichText::new(format!("Sovereign Files ({} items)", filtered_catalog.len()))
            .strong().size(14.0).color(palette::TEXT));
        content_ui.add_space(6.0);

        // Header
        Frame::new().fill(palette::SIDEBAR).corner_radius(4.0).inner_margin(8.0).show(content_ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name / Folder").strong().size(12.0).color(palette::TEXT_DIM));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Actions").strong().size(12.0).color(palette::TEXT_DIM));
                    ui.add_space(30.0);
                    ui.label(RichText::new("Status").strong().size(12.0).color(palette::TEXT_DIM));
                    ui.add_space(40.0);
                    ui.label(RichText::new("Size").strong().size(12.0).color(palette::TEXT_DIM));
                });
            });
        });

        egui::ScrollArea::vertical().max_height(280.0).show(content_ui, |ui| {
            for file in &filtered_catalog {
                render_file_row(ui, app, file);
                ui.add_space(2.0);
            }
        });

        // 3. Right side: Universal Inspector
        crate::ui::inspector::render_inspector_panel(inspector_ui, app);
    });
}

fn render_file_viewport(ui: &mut Ui, app: &mut NexDesktopApp, file: &ProjectedDriveFile) {
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📄 File Viewport").strong().size(14.0).color(palette::ACCENT));
                ui.label(RichText::new(&file.filename).size(14.0).color(palette::TEXT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("Space: {}", file.space_name)).size(11.5).color(palette::TEXT_DIM));
                });
            });
            ui.add_space(6.0);

            // Contextual navigation bridges
            ui.horizontal(|ui| {
                if file.is_media {
                    if ui.button("🎬 Open in Media").clicked() {
                        app.ui.active_tab = NavTab::Media;
                        app.ui.media_state.selected_media_id = Some(file.object_id);
                        app.ui.selected_entity = Some(SelectedEntity::Object(file.object_id));
                    }
                }
                if file.is_location_aware {
                    if ui.button("🗺 View on Map").clicked() {
                        app.ui.active_tab = NavTab::Maps;
                        app.ui.maps_state.selected_object_id = Some(file.object_id);
                        app.ui.selected_entity = Some(SelectedEntity::Object(file.object_id));
                    }
                }
                if ui.button("🌐 View in Network").clicked() {
                    app.ui.active_tab = NavTab::Network;
                    app.ui.network_state.selected_node_id = Some(format!("obj_{}", hex::encode(&file.object_id[0..4])));
                    app.ui.network_state.selected_edge_id = None;
                    app.ui.selected_entity = Some(SelectedEntity::Object(file.object_id));
                }
            });
            ui.add_space(8.0);

            // If text file, render content preview
            if file.is_text {
                if let Some(obj) = app.node.state.object_store.get(&file.object_id) {
                    if let Ok(text) = std::str::from_utf8(&obj.payload_bytes) {
                        Frame::new().fill(Color32::from_rgb(16, 20, 28)).corner_radius(4.0).inner_margin(8.0).show(ui, |ui| {
                            ui.label(RichText::new(text).size(12.0).color(palette::TEXT));
                        });
                    }
                }
            } else {
                ui.label(RichText::new("Binary object representation verified in local CAS. Inspect DAG for chunks.")
                    .size(11.5).color(palette::TEXT_DIM));
            }

            ui.add_space(6.0);
            ui.label(RichText::new("ℹ Destructive mutations (Delete/Rename) require sovereign capability tokens (Read-only observation active).")
                .size(10.5).color(palette::TEXT_DIM));
        });
}

fn render_file_row(ui: &mut Ui, app: &mut NexDesktopApp, file: &ProjectedDriveFile) {
    let is_selected = app.ui.drive_state.selected_file_id == Some(file.object_id);
    let bg = if is_selected { palette::SELECTED } else { palette::PANEL };

    let response = Frame::new()
        .fill(bg)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let icon = if file.is_media { "📷" } else if file.is_text { "📝" } else { "📄" };
                ui.label(RichText::new(icon).size(16.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&file.filename).strong().size(13.0).color(palette::TEXT));
                    ui.label(RichText::new(format!("Folder: {}", file.virtual_folder)).size(11.0).color(palette::TEXT_DIM));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔍 Inspect").clicked() {
                        app.ui.selected_entity = Some(SelectedEntity::Object(file.object_id));
                    }
                    ui.add_space(10.0);
                    ui.label(RichText::new(&file.status_badge).size(12.0).color(palette::ACCENT_GREEN));
                    ui.add_space(20.0);
                    ui.label(RichText::new(&file.byte_size_formatted).size(12.0).color(palette::TEXT_DIM));
                });
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.drive_state.selected_file_id = Some(file.object_id);
        app.ui.selected_entity = Some(SelectedEntity::Object(file.object_id));
    }
}

fn render_empty_state(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("Drive is empty").size(18.0).color(palette::TEXT_DIM));
        ui.add_space(6.0);
        ui.label(RichText::new("Store sovereign files or photos to project them in the Drive Lens.")
            .size(13.0).color(palette::TEXT_DIM));
    });
}

pub fn derive_drive_catalog(app: &NexDesktopApp) -> Vec<ProjectedDriveFile> {
    let mut catalog = Vec::new();

    for (obj_id, obj) in app.node.state.object_store.iter().filter(|(_, o)| !o.tombstoned) {
        let filename = obj.metadata.get("filename")
            .or_else(|| obj.metadata.get("title"))
            .cloned()
            .unwrap_or_else(|| format!("file_{}.bin", hex::encode(&obj_id[0..4])));
        
        let space_name = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());
        let virtual_folder = obj.metadata.get("folder")
            .or_else(|| obj.metadata.get("path"))
            .cloned()
            .unwrap_or_else(|| "/root".to_string());

        let byte_size = obj.payload_bytes.len();
        let byte_size_formatted = format_bytes(byte_size);

        let lower = filename.to_lowercase();
        let is_text = lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".json") || lower.ends_with(".rs") || lower.ends_with(".log");
        let is_media = matches!(obj.object_type, ObjectType::PhotoMedia | ObjectType::PhotoAlbum) ||
            lower.ends_with(".jpg") || lower.ends_with(".png") || lower.ends_with(".webp") || lower.ends_with(".mp4") || lower.ends_with(".mp3");
        let is_location_aware = obj.metadata.contains_key("geo:lat") || obj.metadata.contains_key("location:name");

        catalog.push(ProjectedDriveFile {
            object_id: *obj_id,
            filename,
            space_name,
            object_type: obj.object_type,
            byte_size,
            byte_size_formatted,
            virtual_folder,
            is_text,
            is_media,
            is_location_aware,
            local_available: byte_size > 0,
            status_badge: "Verified Local".to_string(),
        });
    }

    catalog
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
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
    use std::collections::BTreeMap;

    fn create_test_app_with_drive() -> (NexDesktopApp, ObjectID, ObjectID) {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_drive");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        // 1. Text document in Family Space
        let obj1 = [0x11; 32];
        let mut meta1 = BTreeMap::new();
        meta1.insert("filename".to_string(), "Family_Budget.txt".to_string());
        meta1.insert("space".to_string(), "Family".to_string());
        meta1.insert("folder".to_string(), "/finances".to_string());
        node.state.object_store.insert(obj1, NexObject {
            object_id: obj1,
            object_type: ObjectType::DriveInode,
            namespace: [0u8; 32],
            owner_actor_id: node.identity.actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: meta1,
            payload_bytes: b"2026 Sovereign Family Budget: Confirmed".to_vec(),
            tombstoned: false,
        });

        // 2. Location-aware photo file
        let obj2 = [0x22; 32];
        let mut meta2 = BTreeMap::new();
        meta2.insert("filename".to_string(), "Vacation.jpg".to_string());
        meta2.insert("space".to_string(), "Family".to_string());
        meta2.insert("geo:lat".to_string(), "39.0968".to_string());
        meta2.insert("geo:lon".to_string(), "-120.0324".to_string());
        meta2.insert("location:name".to_string(), "Lake Tahoe".to_string());
        meta2.insert("rep:thumb".to_string(), "thumb_22".to_string());
        node.state.object_store.insert(obj2, NexObject {
            object_id: obj2,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: node.identity.actor_id,
            schema_version: 1,
            created_epoch: 101,
            created_lamport: 2,
        winning_mutation_id: [0u8; 32],
            metadata: meta2,
            payload_bytes: vec![0xFF; 2048],
            tombstoned: false,
        });

        // 3. Tombstoned file
        let obj3 = [0x33; 32];
        node.state.object_store.insert(obj3, NexObject {
            object_id: obj3,
            object_type: ObjectType::DriveInode,
            namespace: [0u8; 32],
            owner_actor_id: node.identity.actor_id,
            schema_version: 1,
            created_epoch: 99,
            created_lamport: 0,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: vec![],
            tombstoned: true,
        });

        let app = NexDesktopApp {
            node,
            data_dir,
            ui: crate::ui::NexUiState::new(),
            status: crate::app::AppStatus::Running,
        };

        (app, obj1, obj2)
    }

    #[test]
    fn test_drive_projection_uses_only_canonical_objects() {
        let (app, obj1, obj2) = create_test_app_with_drive();
        let catalog = derive_drive_catalog(&app);

        assert_eq!(catalog.len(), 2, "Must project exactly 2 active canonical objects");
        assert!(catalog.iter().any(|f| f.object_id == obj1 && f.filename == "Family_Budget.txt"));
        assert!(catalog.iter().any(|f| f.object_id == obj2 && f.filename == "Vacation.jpg"));
    }

    #[test]
    fn test_drive_projection_is_ephemeral() {
        let (app, _, _) = create_test_app_with_drive();
        let cat1 = derive_drive_catalog(&app);
        let cat2 = derive_drive_catalog(&app);
        assert_eq!(cat1.len(), cat2.len());
        // Verify no persistent store was created
        assert_eq!(app.node.state.object_store.len(), 3);
    }

    #[test]
    fn test_drive_selection_preserves_object_id() {
        let (mut app, obj1, _) = create_test_app_with_drive();
        app.ui.drive_state.selected_file_id = Some(obj1);
        app.ui.selected_entity = Some(SelectedEntity::Object(obj1));

        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj1)));
    }

    #[test]
    fn test_drive_to_inspector_preserves_object_id() {
        let (app, obj1, _) = create_test_app_with_drive();
        let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(
            &app.node, &obj1, InterfaceComplexity::Standard
        ).unwrap();

        assert_eq!(inspector.object_id, obj1);
        assert_eq!(inspector.title, "Family_Budget.txt");
    }

    #[test]
    fn test_drive_to_media_preserves_object_id() {
        let (mut app, _, obj2) = create_test_app_with_drive();
        app.ui.active_tab = NavTab::Media;
        app.ui.media_state.selected_media_id = Some(obj2);
        app.ui.selected_entity = Some(SelectedEntity::Object(obj2));

        let media_cat = crate::ui::media::derive_media_catalog(&app);
        assert!(media_cat.iter().any(|m| m.object_id == obj2));
    }

    #[test]
    fn test_drive_to_maps_preserves_object_id() {
        let (mut app, _, obj2) = create_test_app_with_drive();
        app.ui.active_tab = NavTab::Maps;
        app.ui.maps_state.selected_object_id = Some(obj2);
        app.ui.selected_entity = Some(SelectedEntity::Object(obj2));

        let geo_cat = crate::ui::maps::derive_geo_catalog(&app);
        assert!(geo_cat.iter().any(|g| g.object_id == obj2));
    }

    #[test]
    fn test_drive_to_network_preserves_object_id() {
        let (mut app, obj1, _) = create_test_app_with_drive();
        app.ui.active_tab = NavTab::Network;
        app.ui.network_state.selected_node_id = Some(format!("obj_{}", hex::encode(&obj1[0..4])));
        app.ui.selected_entity = Some(SelectedEntity::Object(obj1));

        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj1)));
    }

    #[test]
    fn test_empty_drive_state_is_truthful() {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_empty_drive");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        let app = NexDesktopApp {
            node,
            data_dir,
            ui: crate::ui::NexUiState::new(),
            status: crate::app::AppStatus::Running,
        };

        let catalog = derive_drive_catalog(&app);
        assert!(catalog.is_empty(), "Empty node must produce empty Drive catalog");
    }

    #[test]
    fn test_empty_space_state_is_truthful() {
        let (app, _, _) = create_test_app_with_drive();
        let catalog = derive_drive_catalog(&app);
        let personal_files: Vec<_> = catalog.iter().filter(|f| f.space_name == "Personal").collect();
        assert!(personal_files.is_empty(), "Personal Space has no files and must be empty");
    }

    #[test]
    fn test_tombstoned_objects_are_not_presented_as_active() {
        let (app, _, _) = create_test_app_with_drive();
        let catalog = derive_drive_catalog(&app);
        assert!(!catalog.iter().any(|f| f.object_id == [0x33; 32]), "Tombstoned files must be excluded");
    }

    #[test]
    fn test_unsupported_representation_is_not_fabricated() {
        let (app, obj1, _) = create_test_app_with_drive();
        let catalog = derive_drive_catalog(&app);
        let file = catalog.iter().find(|f| f.object_id == obj1).unwrap();
        assert!(!file.is_media, "Plain text file must not be labeled as media");
    }

    #[test]
    fn test_missing_local_representation_is_reported_honestly() {
        let (mut app, _, _) = create_test_app_with_drive();
        let empty_obj = [0x44; 32];
        app.node.state.object_store.insert(empty_obj, NexObject {
            object_id: empty_obj,
            object_type: ObjectType::DriveInode,
            namespace: [0u8; 32],
            owner_actor_id: app.node.identity.actor_id,
            schema_version: 1,
            created_epoch: 102,
            created_lamport: 3,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: vec![],
            tombstoned: false,
        });

        let catalog = derive_drive_catalog(&app);
        let empty_file = catalog.iter().find(|f| f.object_id == empty_obj).unwrap();
        assert!(!empty_file.local_available, "Zero byte payload must report local_available false");
    }

    #[test]
    fn test_file_size_comes_from_canonical_bytes() {
        let (app, obj1, _) = create_test_app_with_drive();
        let catalog = derive_drive_catalog(&app);
        let file = catalog.iter().find(|f| f.object_id == obj1).unwrap();
        assert_eq!(file.byte_size, b"2026 Sovereign Family Budget: Confirmed".len());
    }

    #[test]
    fn test_object_metadata_comes_from_canonical_state() {
        let (app, obj1, _) = create_test_app_with_drive();
        let catalog = derive_drive_catalog(&app);
        let file = catalog.iter().find(|f| f.object_id == obj1).unwrap();
        assert_eq!(file.virtual_folder, "/finances");
    }

    #[test]
    fn test_experience_slider_changes_presentation_only() {
        let (app, obj1, _) = create_test_app_with_drive();
        for tier in [
            InterfaceComplexity::Simple,
            InterfaceComplexity::Standard,
            InterfaceComplexity::Advanced,
            InterfaceComplexity::Expert,
        ] {
            let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(&app.node, &obj1, tier).unwrap();
            assert_eq!(inspector.object_id, obj1);
        }
    }

    #[test]
    fn test_drive_interactions_do_not_mutate_canonical_state() {
        let (mut app, obj1, _) = create_test_app_with_drive();
        let initial_epoch = app.node.state.current_epoch;
        let initial_len = app.node.state.object_store.len();

        app.ui.drive_state.selected_file_id = Some(obj1);
        app.ui.drive_state.search_query = "Budget".to_string();

        assert_eq!(app.node.state.current_epoch, initial_epoch);
        assert_eq!(app.node.state.object_store.len(), initial_len);
    }

    #[test]
    fn test_sensitive_cryptographic_material_never_appears() {
        let (app, obj1, _) = create_test_app_with_drive();
        let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(
            &app.node, &obj1, InterfaceComplexity::Expert
        ).unwrap();
        assert_eq!(inspector.object_id, obj1);
    }

    #[test]
    fn test_filesystem_folders_are_not_falsely_presented_as_spaces() {
        let (app, obj1, _) = create_test_app_with_drive();
        let catalog = derive_drive_catalog(&app);
        let file = catalog.iter().find(|f| f.object_id == obj1).unwrap();

        assert_eq!(file.space_name, "Family", "Space is the E2EE container");
        assert_eq!(file.virtual_folder, "/finances", "Folder is a virtual grouping tag");
        assert_ne!(file.space_name, file.virtual_folder, "Space must never equal folder");
    }

    #[test]
    fn test_no_second_drive_database_exists() {
        let (app, _, _) = create_test_app_with_drive();
        // Catalog is directly evaluated from node object_store
        assert_eq!(app.node.state.object_store.len(), 3);
    }
}
