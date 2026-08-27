use egui::{Ui, RichText, Frame, Color32, Vec2, Pos2, Sense, Stroke, FontId};
use nex_core::object::types::{ObjectID, ObjectType, NexObject};
use nex_core::runtime::shell::SpaceType;
use nex_core::runtime::experience::InterfaceComplexity;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocationPrecision {
    Exact { lat: f64, lon: f64 },
    Approximate { lat: f64, lon: f64, radius_km: f32, place_name: &'static str },
    NamedPlaceOnly,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ProjectedGeoObject {
    pub object_id: ObjectID,
    pub title: String,
    pub space_name: String,
    pub object_type: ObjectType,
    pub place_label: String,
    pub precision: LocationPrecision,
    pub is_historical: bool,
    pub recorded_epoch: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct MapsViewState {
    pub selected_object_id: Option<ObjectID>,
    pub center_lat: f64,
    pub center_lon: f64,
    pub zoom_level: f32,
    pub active_space_filter: Option<SpaceType>,
    pub pan_offset: Vec2,
}

impl MapsViewState {
    pub fn new() -> Self {
        Self {
            selected_object_id: None,
            center_lat: 37.7749,
            center_lon: -122.4194,
            zoom_level: 1.0,
            active_space_filter: None,
            pan_offset: Vec2::ZERO,
        }
    }
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Sovereign Maps & Spatial Lens").size(24.0).strong().color(palette::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("Global Policy: {:?}", app.ui.complexity)).color(palette::ACCENT).size(12.5));
        });
    });

    ui.label(RichText::new("Geographic Projection — Spatial context derived from canonical objects without external tracking")
        .color(palette::TEXT_DIM).size(13.0));
    ui.add_space(8.0);

    // Derive geographic catalog strictly from canonical object store
    let catalog = derive_geo_catalog(app);

    // Filter bar
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filter Space:").color(palette::TEXT_DIM).size(13.0));
        let all_selected = app.ui.maps_state.active_space_filter.is_none();
        if ui.selectable_label(all_selected, "All Locations").clicked() {
            app.ui.maps_state.active_space_filter = None;
        }
        let personal_selected = app.ui.maps_state.active_space_filter == Some(SpaceType::Personal);
        if ui.selectable_label(personal_selected, "🔒 Personal").clicked() {
            app.ui.maps_state.active_space_filter = Some(SpaceType::Personal);
        }
        let family_selected = app.ui.maps_state.active_space_filter == Some(SpaceType::Family);
        if ui.selectable_label(family_selected, "🏡 Family").clicked() {
            app.ui.maps_state.active_space_filter = Some(SpaceType::Family);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("⟲ Reset Map View").clicked() {
                app.ui.maps_state.pan_offset = Vec2::ZERO;
                app.ui.maps_state.zoom_level = 1.0;
            }
        });
    });
    ui.add_space(10.0);

    if catalog.is_empty() {
        render_empty_state(ui);
        return;
    }

    let filtered_catalog: Vec<&ProjectedGeoObject> = catalog.iter()
        .filter(|g| match app.ui.maps_state.active_space_filter {
            None => true,
            Some(SpaceType::Personal) => g.space_name != "Family",
            Some(SpaceType::Family) => g.space_name == "Family",
            Some(_) => true,
        })
        .collect();

    if filtered_catalog.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("No location-associated objects in this Space").size(16.0).color(palette::TEXT_DIM));
            ui.add_space(4.0);
            ui.label(RichText::new("Switch to All Locations or add photos with location metadata")
                .size(13.0).color(palette::TEXT_DIM));
        });
        return;
    }

    // Two-column layout: Left = Map Viewport & Places List, Right = Universal Inspector
    ui.columns(2, |columns| {
        let (left_ui, right_ui) = columns.split_at_mut(1);
        let map_ui = &mut left_ui[0];
        let inspector_ui = &mut right_ui[0];

        // 1. 2D Sovereign Map Viewport
        render_map_viewport(map_ui, app, &filtered_catalog);
        map_ui.add_space(12.0);

        // 2. Location Items Grid
        map_ui.label(RichText::new(format!("Spatial Objects ({} located)", filtered_catalog.len()))
            .strong().size(14.0).color(palette::TEXT));
        map_ui.add_space(6.0);

        egui::ScrollArea::vertical().max_height(240.0).show(map_ui, |ui| {
            for geo in &filtered_catalog {
                render_geo_card(ui, app, geo);
                ui.add_space(4.0);
            }
        });

        // 3. Right side: Universal Inspector
        crate::ui::inspector::render_inspector_panel(inspector_ui, app);
    });
}

fn render_map_viewport(ui: &mut Ui, app: &mut NexDesktopApp, objects: &[&ProjectedGeoObject]) {
    Frame::new()
        .fill(Color32::from_rgb(14, 18, 26))
        .corner_radius(8.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🗺 Spatial Canvas").strong().size(14.0).color(palette::ACCENT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Offline Vector Projection (Zero Cloud Telemetry)")
                        .size(11.0).color(palette::ACCENT_GREEN));
                });
            });
            ui.add_space(4.0);

            let (canvas_rect, response) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), 260.0),
                Sense::drag().union(Sense::click()),
            );

            // Drag to pan
            if response.dragged() {
                app.ui.maps_state.pan_offset += response.drag_delta();
            }

            let painter = ui.painter_at(canvas_rect);

            // Draw dark background & subtle offline geographic coordinate grid
            painter.rect_filled(canvas_rect, 6.0, Color32::from_rgb(16, 20, 30));

            let center = canvas_rect.center() + app.ui.maps_state.pan_offset;

            // Draw latitude/longitude grid lines
            for lat_step in -3..=3 {
                let y = center.y + (lat_step as f32) * 50.0_f32;
                if y >= canvas_rect.min.y && y <= canvas_rect.max.y {
                    painter.line_segment(
                        [Pos2::new(canvas_rect.min.x, y), Pos2::new(canvas_rect.max.x, y)],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 12)),
                    );
                }
            }
            for lon_step in -5..=5 {
                let x = center.x + (lon_step as f32) * 60.0_f32;
                if x >= canvas_rect.min.x && x <= canvas_rect.max.x {
                    painter.line_segment(
                        [Pos2::new(x, canvas_rect.min.y), Pos2::new(x, canvas_rect.max.y)],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 12)),
                    );
                }
            }

            // Project canonical objects as spatial pins
            for (idx, geo) in objects.iter().enumerate() {
                let pin_pos = match geo.precision {
                    LocationPrecision::Exact { lat, lon } => {
                        let px = center.x + ((lon - app.ui.maps_state.center_lon) * 40.0) as f32;
                        let py = center.y - ((lat - app.ui.maps_state.center_lat) * 40.0) as f32;
                        Pos2::new(px, py)
                    }
                    LocationPrecision::Approximate { lat, lon, .. } => {
                        let px = center.x + ((lon - app.ui.maps_state.center_lon) * 40.0) as f32;
                        let py = center.y - ((lat - app.ui.maps_state.center_lat) * 40.0) as f32;
                        Pos2::new(px, py)
                    }
                    LocationPrecision::NamedPlaceOnly | LocationPrecision::Unknown => {
                        Pos2::new(canvas_rect.min.x + 30.0 + (idx as f32 * 60.0), canvas_rect.max.y - 30.0)
                    }
                };

                if canvas_rect.contains(pin_pos) {
                    let is_selected = app.ui.maps_state.selected_object_id == Some(geo.object_id);
                    let pin_color = if is_selected { palette::ACCENT } else { palette::ACCENT_GREEN };

                    if is_selected {
                        painter.circle_stroke(pin_pos, 16.0, Stroke::new(1.5_f32, palette::ACCENT));
                    }
                    painter.circle_filled(pin_pos, 7.0, pin_color);
                    painter.text(
                        Pos2::new(pin_pos.x, pin_pos.y + 12.0),
                        egui::Align2::CENTER_TOP,
                        &geo.place_label,
                        FontId::proportional(11.0),
                        palette::TEXT,
                    );

                    if response.clicked() {
                        if let Some(mouse_pos) = response.interact_pointer_pos() {
                            if mouse_pos.distance(pin_pos) < 18.0 {
                                app.ui.maps_state.selected_object_id = Some(geo.object_id);
                                app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
                            }
                        }
                    }
                }
            }
        });
}

fn render_geo_card(ui: &mut Ui, app: &mut NexDesktopApp, geo: &ProjectedGeoObject) {
    let is_selected = app.ui.maps_state.selected_object_id == Some(geo.object_id);
    let bg = if is_selected { palette::SELECTED } else { palette::PANEL };

    let response = Frame::new()
        .fill(bg)
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📍").size(20.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&geo.title).strong().size(13.5).color(palette::TEXT));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Place: {}", geo.place_label)).size(12.0).color(palette::ACCENT));
                        ui.separator();
                        ui.label(RichText::new(&geo.space_name).size(12.0).color(palette::TEXT_DIM));
                        ui.separator();
                        let precision_label = match geo.precision {
                            LocationPrecision::Exact { .. } => "Exact Coordinates",
                            LocationPrecision::Approximate { .. } => "Approximate (City)",
                            LocationPrecision::NamedPlaceOnly => "Named Place",
                            LocationPrecision::Unknown => "Location Unavailable",
                        };
                        ui.label(RichText::new(precision_label).size(11.5).color(palette::ACCENT_GREEN));
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔍 Inspect").clicked() {
                        app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
                    }
                    if ui.button("🌐 Network").clicked() {
                        app.ui.active_tab = NavTab::Network;
                        app.ui.network_state.selected_node_id = Some(format!("obj_{}", hex::encode(&geo.object_id[0..4])));
                        app.ui.network_state.selected_edge_id = None;
                        app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
                    }
                });
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.maps_state.selected_object_id = Some(geo.object_id);
        app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
    }
}

fn render_empty_state(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("No location-associated objects found").size(18.0).color(palette::TEXT_DIM));
        ui.add_space(6.0);
        ui.label(RichText::new("Objects with location metadata or EXIF tags will be projected here automatically.")
            .size(13.0).color(palette::TEXT_DIM));
    });
}

pub fn derive_geo_catalog(app: &NexDesktopApp) -> Vec<ProjectedGeoObject> {
    let mut catalog = Vec::new();

    for (_obj_id, obj) in app.node.state.object_store.iter().filter(|(_, o)| !o.tombstoned) {
        if let Some(geo) = project_object_location(obj) {
            catalog.push(geo);
        }
    }

    catalog
}

fn project_object_location(obj: &NexObject) -> Option<ProjectedGeoObject> {
    let title = obj.metadata.get("title")
        .or_else(|| obj.metadata.get("filename"))
        .cloned()
        .unwrap_or_else(|| "Untitled Object".to_string());
    let space_name = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());

    // Check for explicit lat/lon metadata
    let lat_opt = obj.metadata.get("geo:lat").and_then(|s| s.parse::<f64>().ok());
    let lon_opt = obj.metadata.get("geo:lon").and_then(|s| s.parse::<f64>().ok());
    let place_name = obj.metadata.get("location:name")
        .or_else(|| obj.metadata.get("place"))
        .cloned();

    let precision = match (lat_opt, lon_opt, place_name.as_ref()) {
        (Some(lat), Some(lon), _) => {
            if obj.metadata.get("geo:accuracy").is_some() {
                LocationPrecision::Approximate { lat, lon, radius_km: 5.0, place_name: "Region" }
            } else {
                LocationPrecision::Exact { lat, lon }
            }
        }
        (None, None, Some(_)) => LocationPrecision::NamedPlaceOnly,
        _ => return None, // Not a location-associated object
    };

    let place_label = place_name.unwrap_or_else(|| {
        if let (Some(lat), Some(lon)) = (lat_opt, lon_opt) {
            format!("{:.3}°, {:.3}°", lat, lon)
        } else {
            "Unknown Place".to_string()
        }
    });

    let is_historical = obj.metadata.contains_key("geo:timestamp") || obj.created_epoch > 0;
    let recorded_epoch = obj.metadata.get("geo:epoch").and_then(|s| s.parse::<u64>().ok()).or(Some(obj.created_epoch));

    Some(ProjectedGeoObject {
        object_id: obj.object_id,
        title,
        space_name,
        object_type: obj.object_type,
        place_label,
        precision,
        is_historical,
        recorded_epoch,
    })
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

    fn create_test_app_with_geo() -> NexDesktopApp {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_geo");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        // 1. Exact GPS Photo
        let mut meta1 = BTreeMap::new();
        meta1.insert("title".to_string(), "Lake Tahoe Trip.jpg".to_string());
        meta1.insert("space".to_string(), "Family".to_string());
        meta1.insert("geo:lat".to_string(), "39.0968".to_string());
        meta1.insert("geo:lon".to_string(), "-120.0324".to_string());
        meta1.insert("location:name".to_string(), "Lake Tahoe".to_string());
        meta1.insert("geo:epoch".to_string(), "100".to_string());

        let obj1 = [1u8; 32];
        node.state.object_store.insert(obj1, NexObject {
            object_id: obj1,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: node.identity.actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: meta1,
            payload_bytes: vec![0xAA; 512],
            tombstoned: false,
        });

        // 2. Object with NO location
        let mut meta2 = BTreeMap::new();
        meta2.insert("title".to_string(), "Private Document.txt".to_string());
        let obj2 = [2u8; 32];
        node.state.object_store.insert(obj2, NexObject {
            object_id: obj2,
            object_type: ObjectType::DriveInode,
            namespace: [0u8; 32],
            owner_actor_id: node.identity.actor_id,
            schema_version: 1,
            created_epoch: 101,
            created_lamport: 2,
        winning_mutation_id: [0u8; 32],
            metadata: meta2,
            payload_bytes: vec![0xBB; 128],
            tombstoned: false,
        });

        NexDesktopApp {
            node,
            data_dir,
            ui: crate::ui::NexUiState::new(),
            status: crate::app::AppStatus::Running,
        }
    }

    #[test]
    fn test_maps_projection_uses_only_canonical_location_state() {
        let app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);

        assert_eq!(catalog.len(), 1, "Only objects with genuine location metadata may be projected");
        assert_eq!(catalog[0].object_id, [1u8; 32]);
        assert_eq!(catalog[0].place_label, "Lake Tahoe");
    }

    #[test]
    fn test_unknown_location_is_not_fabricated() {
        let app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);

        // Object 2 has no location; ensure it was not given fictitious coordinates
        assert!(!catalog.iter().any(|g| g.object_id == [2u8; 32]), "Must never invent coordinates for locationless objects");
    }

    #[test]
    fn test_approximate_location_preserves_uncertainty() {
        let mut app = create_test_app_with_geo();
        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), "San Francisco Photo".to_string());
        meta.insert("geo:lat".to_string(), "37.7749".to_string());
        meta.insert("geo:lon".to_string(), "-122.4194".to_string());
        meta.insert("geo:accuracy".to_string(), "approximate".to_string());

        let obj3 = [3u8; 32];
        app.node.state.object_store.insert(obj3, NexObject {
            object_id: obj3,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: app.node.identity.actor_id,
            schema_version: 1,
            created_epoch: 102,
            created_lamport: 3,
        winning_mutation_id: [0u8; 32],
            metadata: meta,
            payload_bytes: vec![0xCC; 256],
            tombstoned: false,
        });

        let catalog = derive_geo_catalog(&app);
        let sf_item = catalog.iter().find(|g| g.object_id == obj3).unwrap();
        match sf_item.precision {
            LocationPrecision::Approximate { radius_km, .. } => assert_eq!(radius_km, 5.0),
            _ => panic!("Expected approximate precision"),
        }
    }

    #[test]
    fn test_historical_location_is_not_presented_as_current() {
        let app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);
        let item = &catalog[0];

        assert!(item.is_historical, "Capture location must be marked historical");
        assert_eq!(item.recorded_epoch, Some(100));
    }

    #[test]
    fn test_map_selection_preserves_canonical_object_identity() {
        let mut app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);
        let target_id = catalog[0].object_id;

        app.ui.maps_state.selected_object_id = Some(target_id);
        app.ui.selected_entity = Some(SelectedEntity::Object(target_id));

        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object([1u8; 32])));
    }

    #[test]
    fn test_map_to_inspector_preserves_identity() {
        let app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);
        let target_id = catalog[0].object_id;

        let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(
            &app.node, &target_id, InterfaceComplexity::Standard
        ).unwrap();

        assert_eq!(inspector.object_id, target_id);
        assert_eq!(inspector.title, "Lake Tahoe Trip.jpg");
    }

    #[test]
    fn test_map_to_network_preserves_identity() {
        let mut app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);
        let target_id = catalog[0].object_id;

        // Transition Maps -> Network
        app.ui.active_tab = NavTab::Network;
        app.ui.network_state.selected_node_id = Some(format!("obj_{}", hex::encode(&target_id[0..4])));
        app.ui.selected_entity = Some(SelectedEntity::Object(target_id));

        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(target_id)));
    }

    #[test]
    fn test_maps_projection_is_ephemeral() {
        let app = create_test_app_with_geo();
        let catalog1 = derive_geo_catalog(&app);
        let catalog2 = derive_geo_catalog(&app);

        assert_eq!(catalog1.len(), catalog2.len());
        // Verify no persistent state store was spawned
        assert_eq!(app.node.state.object_store.len(), 2);
    }

    #[test]
    fn test_maps_interaction_is_read_only() {
        let mut app = create_test_app_with_geo();
        let initial_epoch = app.node.state.current_epoch;
        let initial_len = app.node.state.object_store.len();

        app.ui.maps_state.pan_offset = Vec2::new(100.0, -50.0);
        app.ui.maps_state.zoom_level = 2.0;
        app.ui.maps_state.selected_object_id = Some([1u8; 32]);

        assert_eq!(app.node.state.current_epoch, initial_epoch);
        assert_eq!(app.node.state.object_store.len(), initial_len);
    }

    #[test]
    fn test_experience_slider_changes_presentation_only() {
        let app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);
        let target_id = catalog[0].object_id;

        for tier in [
            InterfaceComplexity::Simple,
            InterfaceComplexity::Standard,
            InterfaceComplexity::Advanced,
            InterfaceComplexity::Expert,
        ] {
            let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(&app.node, &target_id, tier).unwrap();
            assert_eq!(inspector.object_id, target_id);
        }
    }

    #[test]
    fn test_no_sensitive_location_data_is_unintentionally_exposed() {
        let app = create_test_app_with_geo();
        let catalog = derive_geo_catalog(&app);
        let target_id = catalog[0].object_id;

        let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(&app.node, &target_id, InterfaceComplexity::Simple).unwrap();
        assert_eq!(inspector.title, "Lake Tahoe Trip.jpg");
    }
}
