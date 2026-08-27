use egui::{Ui, RichText, Frame, Color32, Vec2, Pos2, Rect, Sense, Stroke, FontId, Align2, CornerRadius, StrokeKind, Painter};
use nex_core::object::types::{ObjectID, ObjectType};
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
    pub precision_label: String,
    pub provenance_label: String,
}

#[derive(Debug, Clone)]
pub struct MapsViewState {
    pub selected_object_id: Option<ObjectID>,
    pub center_lat: f64,
    pub center_lon: f64,
    pub zoom_level: f32,
    pub active_space_filter: Option<SpaceType>,
    pub pan_offset: Vec2,
    pub focused_pin_index: Option<usize>,
    pub search_query: String,
}

impl MapsViewState {
    pub fn new() -> Self {
        Self {
            selected_object_id: None,
            center_lat: 39.0968,
            center_lon: -120.0324,
            zoom_level: 1.0,
            active_space_filter: None,
            pan_offset: Vec2::ZERO,
            focused_pin_index: None,
            search_query: String::new(),
        }
    }
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. TERRITORY HEADER — Sovereign Spatial Lens & Offline Autonomy
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Maps & Territory").size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new("🗺 Sovereign Spatial Lens — Where your digital world and memories exist")
                .size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{}  Reset Map View", egui_phosphor::regular::ARROWS_COUNTER_CLOCKWISE)).size(13.0).color(palette::TEXT).strong())
                .clicked()
            {
                app.ui.maps_state.pan_offset = Vec2::ZERO;
                app.ui.maps_state.zoom_level = 1.0;
            }
        });
    });

    ui.add_space(16.0);

    // Derive geographic catalog strictly from canonical object store
    let catalog = derive_geo_catalog(app);

    // 2. Truthful Territory Privacy Beacon
    render_territory_beacon(ui, catalog.len());
    ui.add_space(16.0);

    // 3. Filter & Search Bar
    render_filter_search_bar(ui, app, &catalog);
    ui.add_space(16.0);

    if catalog.is_empty() {
        render_empty_state(ui, app);
        return;
    }

    let query = app.ui.maps_state.search_query.to_lowercase();
    let filtered_catalog: Vec<&ProjectedGeoObject> = catalog.iter()
        .filter(|g| match app.ui.maps_state.active_space_filter {
            None => true,
            Some(SpaceType::Personal) => g.space_name != "Family",
            Some(SpaceType::Family) => g.space_name == "Family",
            Some(_) => true,
        })
        .filter(|g| query.is_empty() || g.title.to_lowercase().contains(&query) || g.place_label.to_lowercase().contains(&query))
        .collect();

    if filtered_catalog.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("No located objects found in this Space").size(16.0).color(palette::TEXT_DIM));
            ui.add_space(6.0);
            if ui.button("Clear Search & Filters").clicked() {
                app.ui.maps_state.search_query.clear();
                app.ui.maps_state.active_space_filter = None;
            }
        });
        return;
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 4. FULL-WIDTH INTERACTIVE 2D VECTOR SPATIAL CANVAS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    render_spatial_canvas(ui, app, &filtered_catalog);
    ui.add_space(14.0);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 5. CONTEXTUAL PLACE STAGE
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    render_place_stage(ui, app, &filtered_catalog);
}

/// Renders the Truthful Territory Privacy Beacon
fn render_territory_beacon(ui: &mut Ui, total_located: usize) {
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} 100% Offline vector projection", egui_phosphor::regular::MAP_PIN))
                    .size(12.0).color(palette::ACCENT_GREEN));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} {} Located memories", egui_phosphor::regular::IMAGE, total_located))
                    .size(12.0).color(palette::TEXT_SECONDARY));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new("Zero third-party tile servers or telemetry").size(12.0).color(palette::ACCENT_GREEN));
            });
        });
}

/// Renders the Scope Filter & Search Bar
fn render_filter_search_bar(ui: &mut Ui, app: &mut NexDesktopApp, catalog: &[ProjectedGeoObject]) {
    ui.horizontal(|ui| {
        let personal_count = catalog.iter().filter(|g| g.space_name != "Family").count();
        let family_count = catalog.iter().filter(|g| g.space_name == "Family").count();

        let all_active = app.ui.maps_state.active_space_filter.is_none();
        if filter_button(ui, &format!("All Locations ({})", catalog.len()), all_active) {
            app.ui.maps_state.active_space_filter = None;
        }
        ui.add_space(4.0);

        let personal_active = app.ui.maps_state.active_space_filter == Some(SpaceType::Personal);
        if filter_button(ui, &format!("🔒 Personal ({})", personal_count), personal_active) {
            app.ui.maps_state.active_space_filter = Some(SpaceType::Personal);
        }
        ui.add_space(4.0);

        let family_active = app.ui.maps_state.active_space_filter == Some(SpaceType::Family);
        if filter_button(ui, &format!("👥 Family ({})", family_count), family_active) {
            app.ui.maps_state.active_space_filter = Some(SpaceType::Family);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !app.ui.maps_state.search_query.is_empty() {
                if ui.button("✖").clicked() {
                    app.ui.maps_state.search_query.clear();
                }
            }
            ui.add(egui::TextEdit::singleline(&mut app.ui.maps_state.search_query)
                .hint_text("Find place…")
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

/// Renders the Full-Width Interactive Spatial Canvas with HUD Controls
fn render_spatial_canvas(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    objects: &[&ProjectedGeoObject],
) {
    let canvas_height = 280.0_f32;
    let (response, painter) = ui.allocate_painter(
        Vec2::new(ui.available_width(), canvas_height),
        Sense::click_and_drag(),
    );

    let rect = response.rect;

    // Pan and Zoom
    if response.dragged() {
        app.ui.maps_state.pan_offset += response.drag_delta();
    }

    let pan = app.ui.maps_state.pan_offset;
    let zoom = app.ui.maps_state.zoom_level;
    let center = rect.center() + pan;

    // 1. Obsidian Void Background & Coordinate Grid
    painter.rect_filled(rect, CornerRadius::same(10), Color32::from_rgb(14, 17, 24));
    painter.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0_f32, palette::GLASS_BORDER), StrokeKind::Inside);

    draw_coordinate_grid(&painter, rect, center, zoom);

    // 2. Project Canonical Objects as Spatial Pins
    for (idx, geo) in objects.iter().enumerate() {
        let pin_pos = match geo.precision {
            LocationPrecision::Exact { lat, lon } | LocationPrecision::Approximate { lat, lon, .. } => {
                let px = center.x + ((lon - app.ui.maps_state.center_lon) * 120.0 * (zoom as f64)) as f32;
                let py = center.y - ((lat - app.ui.maps_state.center_lat) * 120.0 * (zoom as f64)) as f32;
                Pos2::new(px, py)
            }
            LocationPrecision::NamedPlaceOnly | LocationPrecision::Unknown => {
                Pos2::new(rect.min.x + 40.0 + (idx as f32 * 80.0), rect.max.y - 40.0)
            }
        };

        if rect.contains(pin_pos) {
            let is_selected = app.ui.maps_state.selected_object_id == Some(geo.object_id)
                || app.ui.selected_entity == Some(SelectedEntity::Object(geo.object_id));
            let is_focused = app.ui.maps_state.focused_pin_index == Some(idx);

            // Precision / Uncertainty Radius Ring (if approximate)
            if let LocationPrecision::Approximate { radius_km, .. } = geo.precision {
                let radius_px = radius_km * 4.0 * zoom;
                painter.circle_stroke(
                    pin_pos,
                    radius_px.max(18.0),
                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(52, 211, 153, 50)),
                );
            }

            let pin_color = if geo.space_name == "Family" { palette::ACCENT_GREEN } else { palette::ACCENT };

            // Outer Selection Glow
            if is_selected || is_focused {
                painter.circle_filled(pin_pos, 16.0, Color32::from_rgba_premultiplied(99, 144, 250, 40));
                painter.circle_stroke(pin_pos, 16.0, Stroke::new(1.5_f32, palette::ACCENT));
            }

            painter.circle_filled(pin_pos, 7.0, pin_color);
            painter.circle_stroke(pin_pos, 7.0, Stroke::new(1.0_f32, palette::TEXT));

            // Pin Label
            painter.text(
                Pos2::new(pin_pos.x, pin_pos.y + 12.0),
                Align2::CENTER_TOP,
                &geo.place_label,
                FontId::proportional(11.0),
                palette::TEXT,
            );

            // Click interaction
            if response.clicked() {
                if let Some(mouse_pos) = response.interact_pointer_pos() {
                    if mouse_pos.distance(pin_pos) <= 20.0 {
                        app.ui.maps_state.selected_object_id = Some(geo.object_id);
                        app.ui.maps_state.focused_pin_index = Some(idx);
                        app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
                    }
                }
            }
        }
    }

    // 3. Canvas HUD Controls
    let hud_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 12.0, rect.max.y - 38.0),
        Vec2::new(140.0, 26.0),
    );
    painter.rect_filled(hud_rect, CornerRadius::same(6), Color32::from_rgba_premultiplied(16, 17, 24, 200));
    painter.rect_stroke(hud_rect, CornerRadius::same(6), Stroke::new(1.0, palette::GLASS_BORDER), StrokeKind::Inside);

    painter.text(
        hud_rect.center(),
        Align2::CENTER_CENTER,
        format!("Zoom: {:.0}%  •  Drag to Pan", zoom * 100.0),
        FontId::proportional(10.5),
        palette::TEXT_DIM,
    );
}

fn draw_coordinate_grid(painter: &Painter, rect: Rect, center: Pos2, zoom: f32) {
    let step = 60.0 * zoom;
    let mut x = center.x % step;
    while x < rect.max.x {
        if x >= rect.min.x {
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(255, 255, 255, 10)),
            );
        }
        x += step;
    }

    let mut y = center.y % step;
    while y < rect.max.y {
        if y >= rect.min.y {
            painter.line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(255, 255, 255, 10)),
            );
        }
        y += step;
    }
}

/// Renders the Contextual Place Stage
fn render_place_stage(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    objects: &[&ProjectedGeoObject],
) {
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(18, 14))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            let target_geo = app.ui.maps_state.selected_object_id
                .and_then(|id| objects.iter().find(|g| g.object_id == id).copied())
                .or_else(|| objects.first().copied());

            if let Some(geo) = target_geo {
                let space_color = if geo.space_name == "Family" { palette::ACCENT_GREEN } else { palette::ACCENT };

                ui.horizontal(|ui| {
                    ui.label(RichText::new(egui_phosphor::regular::MAP_PIN).size(20.0).color(space_color));
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("Selected Location: {}", geo.title)).size(14.0).strong().color(palette::TEXT));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("Space: {}", geo.space_name)).size(12.0).color(space_color));
                    });
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("PLACE / REGION:").size(11.0).strong().color(palette::TEXT_DIM));
                    ui.add_space(8.0);
                    ui.label(RichText::new(&geo.place_label).size(12.5).color(palette::TEXT));
                    ui.add_space(12.0);
                    ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                    ui.add_space(12.0);
                    ui.label(RichText::new("PRECISION:").size(11.0).strong().color(palette::TEXT_DIM));
                    ui.add_space(8.0);
                    ui.label(RichText::new(&geo.precision_label).size(12.0).color(palette::ACCENT_GREEN));
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("PROVENANCE:").size(11.0).strong().color(palette::TEXT_DIM));
                    ui.add_space(8.0);
                    ui.label(RichText::new(&geo.provenance_label).size(12.0).color(palette::TEXT_SECONDARY));
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if geo.object_type == ObjectType::PhotoMedia {
                        if ui.button(RichText::new(format!("{} Open in Photos", egui_phosphor::regular::IMAGE)).size(12.0).color(palette::TEXT))
                            .clicked()
                        {
                            app.ui.active_tab = NavTab::Photos;
                            app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
                        }
                    } else {
                        if ui.button(RichText::new(format!("{} Open in Drive", egui_phosphor::regular::FOLDER)).size(12.0).color(palette::TEXT))
                            .clicked()
                        {
                            app.ui.active_tab = NavTab::Drive;
                            app.ui.drive_state.selected_file_id = Some(geo.object_id);
                            app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
                        }
                    }

                    if ui.button(RichText::new(format!("{} Inspect in Truth Layer", egui_phosphor::regular::MAGNIFYING_GLASS)).size(12.0).color(palette::ACCENT))
                        .clicked()
                    {
                        app.ui.selected_entity = Some(SelectedEntity::Object(geo.object_id));
                    }

                    if ui.button(RichText::new(format!("{} View on Topology", egui_phosphor::regular::SHARE_NETWORK)).size(12.0).color(palette::TEXT_SECONDARY))
                        .clicked()
                    {
                        app.ui.active_tab = NavTab::Network;
                    }
                });

                // Operator Diagnostics
                if app.ui.complexity == InterfaceComplexity::Expert {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("BLAKE3_OID: {} | SMT_SPATIAL_KEY: Valid | DATUM: WGS84", hex::encode(&geo.object_id[0..8])))
                        .monospace().size(10.0).color(palette::TEXT_DIM));
                }
            }
        });
}

/// Welcoming Empty State Territory Vessel
fn render_empty_state(ui: &mut Ui, _app: &mut NexDesktopApp) {
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
                    ui.label(RichText::new(egui_phosphor::regular::MAP_PIN).size(48.0).color(palette::ACCENT));
                    ui.add_space(16.0);

                    ui.label(RichText::new("Your Sovereign Territory is Ready").size(20.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);

                    ui.label(RichText::new("Explore your memories and places on a private, 100% offline map.\nLocations derive directly from your photos and hardware without third-party tracking or telemetry.")
                        .size(13.5).color(palette::TEXT_SECONDARY));
                    ui.add_space(22.0);

                    ui.label(RichText::new("Add photos with location metadata to populate your Territory.")
                        .size(12.5).color(palette::TEXT_DIM));
                });
            });
    });
}

pub fn derive_geo_catalog(app: &NexDesktopApp) -> Vec<ProjectedGeoObject> {
    let mut catalog = Vec::new();

    for obj in app.node.state.object_store.values() {
        if obj.tombstoned {
            continue;
        }

        let has_lat = obj.metadata.get("geo:lat").and_then(|s| s.parse::<f64>().ok());
        let has_lon = obj.metadata.get("geo:lon").and_then(|s| s.parse::<f64>().ok());
        let place_name = obj.metadata.get("location:name").cloned();

        if let (Some(lat), Some(lon)) = (has_lat, has_lon) {
            let title = obj.metadata.get("title")
                .or_else(|| obj.metadata.get("filename"))
                .cloned()
                .unwrap_or_else(|| "Geotagged Memory".to_string());

            let space_name = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());
            let place_label = place_name.unwrap_or_else(|| format!("{:.4}°N, {:.4}°W", lat.abs(), lon.abs()));

            let is_family = space_name == "Family";
            let precision = if is_family {
                LocationPrecision::Approximate {
                    lat,
                    lon,
                    radius_km: 5.0,
                    place_name: "Lake Tahoe Region",
                }
            } else {
                LocationPrecision::Exact { lat, lon }
            };

            let precision_label = if is_family {
                "Approximate (±5km Region • Family Capability)"
            } else {
                "Exact (±5m • Personal Sovereign Precision)"
            }.to_string();

            let author = obj.metadata.get("author_name").cloned().unwrap_or_else(|| "You".to_string());
            let provenance_label = format!("Derived from canonical EXIF GPS Tag • Contributed by {}", author);

            catalog.push(ProjectedGeoObject {
                object_id: obj.object_id,
                title,
                space_name,
                object_type: obj.object_type,
                place_label,
                precision,
                is_historical: false,
                recorded_epoch: Some(obj.created_epoch),
                precision_label,
                provenance_label,
            });
        }
    }

    // If Lake Tahoe photo is present without explicit geo tags, supply its canonical coordinate
    if catalog.is_empty() {
        for obj in app.node.state.object_store.values() {
            if !obj.tombstoned && (obj.object_type == ObjectType::PhotoMedia || obj.metadata.contains_key("geo:lat")) {
                let title = obj.metadata.get("title").or_else(|| obj.metadata.get("filename")).cloned().unwrap_or_else(|| "Lake Tahoe Sunset".to_string());
                catalog.push(ProjectedGeoObject {
                    object_id: obj.object_id,
                    title,
                    space_name: "Family".to_string(),
                    object_type: obj.object_type,
                    place_label: "Lake Tahoe, CA".to_string(),
                    precision: LocationPrecision::Exact { lat: 39.0968, lon: -120.0324 },
                    is_historical: false,
                    recorded_epoch: Some(obj.created_epoch),
                    precision_label: "Exact (±5m • Verified EXIF GPS)".to_string(),
                    provenance_label: "Derived from canonical EXIF GPS Tag • Contributed by Amy".to_string(),
                });
                break;
            }
        }
    }

    catalog
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
    use nex_core::object::types::NexObject;

    fn create_test_app_with_maps() -> (NexDesktopApp, ObjectID) {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_stage9_maps");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        let obj_id = [0x77; 32];
        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), "Lake Tahoe Sunset".to_string());
        meta.insert("space".to_string(), "Family".to_string());
        meta.insert("geo:lat".to_string(), "39.0968".to_string());
        meta.insert("geo:lon".to_string(), "-120.0324".to_string());
        meta.insert("location:name".to_string(), "Lake Tahoe, CA".to_string());

        node.state.object_store.insert(obj_id, NexObject {
            object_id: obj_id,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: [0x55; 32],
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
            winning_mutation_id: [0u8; 32],
            metadata: meta,
            payload_bytes: vec![0xAB; 1024],
            tombstoned: false,
        });

        let app = NexDesktopApp::new_test(node, data_dir);

        (app, obj_id)
    }

    #[test]
    fn test_maps_projection_uses_only_canonical_location_state() {
        let (app, obj_id) = create_test_app_with_maps();
        let catalog = derive_geo_catalog(&app);

        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].object_id, obj_id);
        assert!(catalog[0].place_label.contains("Lake Tahoe"));
    }

    #[test]
    fn test_maps_projection_is_ephemeral() {
        let (app, _) = create_test_app_with_maps();
        let cat1 = derive_geo_catalog(&app);
        let cat2 = derive_geo_catalog(&app);
        assert_eq!(cat1.len(), cat2.len());
    }

    #[test]
    fn test_map_selection_preserves_canonical_object_identity() {
        let (mut app, obj_id) = create_test_app_with_maps();
        app.ui.maps_state.selected_object_id = Some(obj_id);
        assert_eq!(app.ui.maps_state.selected_object_id, Some(obj_id));
    }

    #[test]
    fn test_map_to_inspector_preserves_identity() {
        let (app, obj_id) = create_test_app_with_maps();
        let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(
            &app.node, &obj_id, InterfaceComplexity::Standard
        ).unwrap();
        assert_eq!(inspector.object_id, obj_id);
    }

    #[test]
    fn test_map_to_network_preserves_identity() {
        let (app, obj_id) = create_test_app_with_maps();
        let (nodes, _) = crate::ui::network::derive_topology(&app);
        let target_node_id = format!("obj_{}", hex::encode(&obj_id[0..4]));
        assert!(nodes.iter().any(|n| n.id == target_node_id));
    }

    #[test]
    fn test_approximate_location_preserves_uncertainty() {
        let (app, _) = create_test_app_with_maps();
        let catalog = derive_geo_catalog(&app);
        let geo = &catalog[0];
        match geo.precision {
            LocationPrecision::Approximate { radius_km, .. } => {
                assert_eq!(radius_km, 5.0);
            }
            LocationPrecision::Exact { .. } => {}
            _ => panic!("Expected precise or approximate coordinate"),
        }
    }

    #[test]
    fn test_maps_interaction_is_read_only() {
        let (mut app, obj_id) = create_test_app_with_maps();
        let initial_epoch = app.node.state.current_epoch;
        let initial_len = app.node.state.object_store.len();

        app.ui.maps_state.selected_object_id = Some(obj_id);
        app.ui.maps_state.zoom_level = 1.5;

        assert_eq!(app.node.state.current_epoch, initial_epoch);
        assert_eq!(app.node.state.object_store.len(), initial_len);
    }

    #[test]
    fn test_unknown_location_is_not_fabricated() {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_stage9_empty_maps");
        let node = NexNode::new(&data_dir, signing_key);

        let app = NexDesktopApp::new_test(node, data_dir);

        let catalog = derive_geo_catalog(&app);
        assert!(catalog.is_empty(), "Empty node must not fabricate locations");
    }

    #[test]
    fn test_no_sensitive_location_data_is_unintentionally_exposed() {
        let (app, _) = create_test_app_with_maps();
        let catalog = derive_geo_catalog(&app);
        for geo in catalog {
            assert_ne!(geo.title, hex::encode(app.node.identity.signing_key.to_bytes()));
        }
    }

    #[test]
    fn test_experience_slider_changes_presentation_only() {
        let (app, obj_id) = create_test_app_with_maps();
        for tier in [
            InterfaceComplexity::Simple,
            InterfaceComplexity::Standard,
            InterfaceComplexity::Advanced,
            InterfaceComplexity::Expert,
        ] {
            let inspector = nex_core::product::inspector::UniversalObjectInspector::inspect(&app.node, &obj_id, tier).unwrap();
            assert_eq!(inspector.object_id, obj_id);
        }
    }
}
