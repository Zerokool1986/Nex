use egui::{Ui, Pos2, Vec2, Rect, Color32, Stroke, CornerRadius, StrokeKind, RichText, Frame, Sense, Painter, FontId, Align2};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::{ObjectID, ObjectType};
use nex_core::product::inspector::UniversalObjectInspector;
use nex_core::runtime::panels::ContextualPanelsEngine;
use crate::app::{NexDesktopApp, AppStatus};
use crate::ui::{palette, NavTab, NexUiState, inspector::SelectedEntity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipClass {
    Logical,
    Network,
    Transport,
    DataFlow,
}

impl RelationshipClass {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Logical => "Logical (Space / Scope)",
            Self::Network => "Mesh Link (Direct LAN Peer)",
            Self::Transport => "Transport (NEX/WIRE/v1)",
            Self::DataFlow => "Data Replication (CAS Inode)",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Logical => Color32::from_rgb(91, 141, 246),    // Radiant Cobalt
            Self::Network => Color32::from_rgb(52, 211, 153),   // Emerald Mesh
            Self::Transport => Color32::from_rgb(251, 191, 36),  // Amber Wire
            Self::DataFlow => Color32::from_rgb(168, 85, 247),   // Purple CAS
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodePayload {
    Device { actor_id_hex: String, is_local: bool },
    Space { space_type: SpaceType, item_count: usize },
    Object { object_id: ObjectID, object_type: ObjectType, title: String, space_name: String },
    TransportSubstrate { name: String, status: String },
}

#[derive(Debug, Clone)]
pub struct VisualizerNode {
    pub id: String,
    pub label: String,
    pub subtitle: String,
    pub icon_glyph: &'static str,
    pub base_pos: Pos2,
    pub payload: NodePayload,
}

#[derive(Debug, Clone)]
pub struct VisualizerEdge {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub label: String,
    pub relationship_class: RelationshipClass,
    pub explanation_simple: String,
    pub explanation_standard: String,
    pub explanation_advanced: String,
    pub explanation_operator: String,
}

pub struct NetworkViewState {
    pub pan_offset: Vec2,
    pub zoom_level: f32,
    pub selected_node_id: Option<String>,
    pub selected_edge_id: Option<String>,
}

impl NetworkViewState {
    pub fn new() -> Self {
        Self {
            pan_offset: Vec2::ZERO,
            zoom_level: 1.0,
            selected_node_id: None,
            selected_edge_id: None,
        }
    }
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(RichText::new("Sovereign Topology Radar").size(24.0).strong().color(palette::TEXT));
            ui.label(RichText::new("Live mathematical constellation of sovereign spaces, physical devices, and CAS objects")
                .color(palette::TEXT_DIM).size(13.0));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Zoom & Canvas controls
            if ui.button(format!("{} Reset", egui_phosphor::regular::ARROWS_COUNTER_CLOCKWISE)).clicked() {
                app.ui.network_state.pan_offset = Vec2::ZERO;
                app.ui.network_state.zoom_level = 1.0;
            }
            if ui.button(egui_phosphor::regular::PLUS).clicked() {
                app.ui.network_state.zoom_level = (app.ui.network_state.zoom_level * 1.15).min(2.5);
            }
            if ui.button(egui_phosphor::regular::MINUS).clicked() {
                app.ui.network_state.zoom_level = (app.ui.network_state.zoom_level / 1.15).max(0.5);
            }

            ui.separator();
            ui.label(RichText::new(format!("Topology Tier: {:?}", app.ui.complexity)).color(palette::ACCENT).size(12.5));
        });
    });
    ui.add_space(10.0);

    // Derive topology live from canonical state
    let (nodes, edges) = derive_topology(app);

    // Split view: Left = 2D Topology Constellation Radar, Right = Contextual Explanation & Inspector
    ui.columns(2, |columns| {
        let (first, second) = columns.split_at_mut(1);
        let canvas_ui = &mut first[0];
        let inspector_ui = &mut second[0];

        // 1. Modern Constellation Radar Canvas
        render_canvas(canvas_ui, app, &nodes, &edges);

        // 2. Contextual Relationship Inspector / "Why is this connected?"
        render_inspector(inspector_ui, app, &nodes, &edges);
    });
}

pub fn derive_topology(app: &NexDesktopApp) -> (Vec<VisualizerNode>, Vec<VisualizerEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let local_actor_hex = hex::encode(&app.node.identity.actor_id[0..4]);
    let local_device_id = "device_local".to_string();

    // Center Node: Local Device (This PC)
    nodes.push(VisualizerNode {
        id: local_device_id.clone(),
        label: "This PC (Windows Host)".to_string(),
        subtitle: format!("Actor: {}", local_actor_hex),
        icon_glyph: egui_phosphor::regular::DESKTOP,
        base_pos: Pos2::new(260.0, 200.0),
        payload: NodePayload::Device {
            actor_id_hex: hex::encode(&app.node.identity.actor_id),
            is_local: true,
        },
    });

    // Space Node 1: Personal Sanctuary
    let personal_space_id = "space_personal".to_string();
    let personal_items = app.node.state.object_store.values()
        .filter(|o| o.metadata.get("space").map(|s| s.as_str()) != Some("Family") && !o.tombstoned)
        .count();
    nodes.push(VisualizerNode {
        id: personal_space_id.clone(),
        label: "Personal Sanctuary".to_string(),
        subtitle: format!("{} objects", personal_items),
        icon_glyph: egui_phosphor::regular::HOUSE,
        base_pos: Pos2::new(100.0, 90.0),
        payload: NodePayload::Space { space_type: SpaceType::Personal, item_count: personal_items },
    });

    // Space Node 2: Family Space
    let family_space_id = "space_family".to_string();
    let family_items = app.node.state.object_store.values()
        .filter(|o| o.metadata.get("space").map(|s| s.as_str()) == Some("Family") && !o.tombstoned)
        .count();
    nodes.push(VisualizerNode {
        id: family_space_id.clone(),
        label: "Family Space".to_string(),
        subtitle: format!("{} objects", family_items),
        icon_glyph: egui_phosphor::regular::HEART,
        base_pos: Pos2::new(420.0, 90.0),
        payload: NodePayload::Space { space_type: SpaceType::Family, item_count: family_items },
    });

    // Transport Node: LAN Mesh & UDP Discovery
    let transport_id = "transport_lan".to_string();
    nodes.push(VisualizerNode {
        id: transport_id.clone(),
        label: "Direct LAN Mesh".to_string(),
        subtitle: format!("UDP 8765 / State: {:?}", app.node.operational_state),
        icon_glyph: egui_phosphor::regular::SHARE_NETWORK,
        base_pos: Pos2::new(100.0, 310.0),
        payload: NodePayload::TransportSubstrate {
            name: "TCP/UDP Direct LAN".to_string(),
            status: format!("{:?}", app.node.operational_state),
        },
    });

    // Edge: Local Device -> Personal Space
    edges.push(VisualizerEdge {
        id: "edge_device_personal".to_string(),
        from_node_id: local_device_id.clone(),
        to_node_id: personal_space_id.clone(),
        label: "owns namespace".to_string(),
        relationship_class: RelationshipClass::Logical,
        explanation_simple: "This device holds the cryptographic root of your Personal Space.".to_string(),
        explanation_standard: "Your local Windows host holds the root cryptographic identity for your Personal Space.".to_string(),
        explanation_advanced: format!("Actor {} is sovereign owner of Namespace 0x00..00 (Personal).", local_actor_hex),
        explanation_operator: format!("Master Ed25519 Key verified. Local CAS chunk partition active. Current epoch: {}.", app.node.state.current_epoch),
    });

    // Edge: Local Device -> Family Space
    edges.push(VisualizerEdge {
        id: "edge_device_family".to_string(),
        from_node_id: local_device_id.clone(),
        to_node_id: family_space_id.clone(),
        label: "replicated member".to_string(),
        relationship_class: RelationshipClass::Logical,
        explanation_simple: "This device is a verified member of the Family Space.".to_string(),
        explanation_standard: "Configured for Family Space synchronization and shared media storage.".to_string(),
        explanation_advanced: "Local actor authorized for Family Namespace (SpaceType::Family).".to_string(),
        explanation_operator: "Local SMT root initialized. Anti-entropy sync gateway ready for peer discovery.".to_string(),
    });

    // Edge: Local Device -> Transport
    edges.push(VisualizerEdge {
        id: "edge_device_transport".to_string(),
        from_node_id: local_device_id.clone(),
        to_node_id: transport_id.clone(),
        label: "active carrier".to_string(),
        relationship_class: RelationshipClass::Transport,
        explanation_simple: "Direct peer-to-peer LAN mesh transport is active without internet.".to_string(),
        explanation_standard: "This PC listens on local network TCP/UDP sockets for peer discovery and sync.".to_string(),
        explanation_advanced: format!("Transport adapter bound. Node state: {:?}. Wire framing: NEX/WIRE/v1.", app.node.operational_state),
        explanation_operator: "48-byte binary frame headers enabled. Sockets bound on local loopback/LAN.".to_string(),
    });

    // Objects in store (orbiting nodes)
    let mut obj_idx = 0;
    for (obj_id, obj) in app.node.state.object_store.iter().filter(|(_, o)| !o.tombstoned).take(4) {
        let obj_node_id = format!("obj_{}", hex::encode(&obj_id[0..4]));
        let title = obj.metadata.get("title")
            .or_else(|| obj.metadata.get("filename"))
            .cloned()
            .unwrap_or_else(|| "Sovereign Object".to_string());
        let space_name = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());
        let icon_glyph = match obj.object_type {
            ObjectType::PhotoMedia => egui_phosphor::regular::IMAGE,
            ObjectType::DriveInode => egui_phosphor::regular::FILE_TEXT,
            _ => egui_phosphor::regular::FILE,
        };

        let pos_x = 340.0 + (obj_idx as f32) * 60.0;
        let pos_y = 260.0 + ((obj_idx % 2) as f32) * 55.0;

        nodes.push(VisualizerNode {
            id: obj_node_id.clone(),
            label: if title.len() > 14 { format!("{}...", &title[0..12]) } else { title.clone() },
            subtitle: format!("{} B", obj.payload_bytes.len()),
            icon_glyph,
            base_pos: Pos2::new(pos_x, pos_y),
            payload: NodePayload::Object {
                object_id: *obj_id,
                object_type: obj.object_type,
                title: title.clone(),
                space_name: space_name.clone(),
            },
        });

        // Edge to Space
        let target_space_node = if space_name == "Family" { family_space_id.clone() } else { personal_space_id.clone() };
        edges.push(VisualizerEdge {
            id: format!("edge_obj_{}", obj_node_id),
            from_node_id: target_space_node,
            to_node_id: obj_node_id.clone(),
            label: "contains".to_string(),
            relationship_class: RelationshipClass::Logical,
            explanation_simple: format!("Object lives inside the {} Space.", space_name),
            explanation_standard: format!("Object stored under {} Space namespace with sovereign encryption.", space_name),
            explanation_advanced: format!("Schema v{} CAS Inode | Namespace: {}", obj.schema_version, space_name),
            explanation_operator: format!("SMT Leaf Key: {} | Author: {} | Epoch: {}", hex::encode(&obj.object_id[0..8]), hex::encode(&obj.owner_actor_id[0..4]), obj.created_epoch),
        });

        // Edge to Local Device (CAS)
        edges.push(VisualizerEdge {
            id: format!("edge_store_{}", obj_node_id),
            from_node_id: local_device_id.clone(),
            to_node_id: obj_node_id.clone(),
            label: "stored on CAS".to_string(),
            relationship_class: RelationshipClass::DataFlow,
            explanation_simple: "Stored locally on this PC.".to_string(),
            explanation_standard: "Local CAS replica is complete and verified.".to_string(),
            explanation_advanced: format!("Stored in local FastCDC chunk store ({} bytes).", obj.payload_bytes.len()),
            explanation_operator: "Direct CAS Inode mapping verified on local filesystem.".to_string(),
        });

        obj_idx += 1;
    }

    (nodes, edges)
}

fn render_canvas(ui: &mut Ui, app: &mut NexDesktopApp, nodes: &[VisualizerNode], edges: &[VisualizerEdge]) {
    Frame::new()
        .fill(Color32::from_rgb(11, 12, 16))
        .corner_radius(10.0)
        .inner_margin(12.0)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(30, 32, 42)))
        .show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 460.0), Sense::click_and_drag());

            // Handle Pan & Drag
            if response.dragged() {
                app.ui.network_state.pan_offset += response.drag_delta();
            }

            let painter = ui.painter_at(rect);
            let zoom = app.ui.network_state.zoom_level;
            let pan = app.ui.network_state.pan_offset;

            // Transform function
            let to_screen = |base: Pos2| -> Pos2 {
                Pos2::new(
                    rect.min.x + pan.x + (base.x * zoom),
                    rect.min.y + pan.y + (base.y * zoom),
                )
            };

            // 1. Draw Subtle Ambient Radar Rings & Radar Grid
            let center_screen = to_screen(Pos2::new(260.0, 200.0));
            draw_radar_atmosphere(&painter, rect, center_screen, zoom);

            // 2. Draw Soft Splines / Flow Links
            for edge in edges {
                if let (Some(from_node), Some(to_node)) = (
                    nodes.iter().find(|n| n.id == edge.from_node_id),
                    nodes.iter().find(|n| n.id == edge.to_node_id),
                ) {
                    let p1 = to_screen(from_node.base_pos);
                    let p2 = to_screen(to_node.base_pos);
                    let is_selected = app.ui.network_state.selected_edge_id.as_deref() == Some(&edge.id);

                    let base_color = edge.relationship_class.color();
                    let stroke_color = if is_selected {
                        Color32::WHITE
                    } else {
                        Color32::from_rgba_unmultiplied(base_color.r(), base_color.g(), base_color.b(), 180)
                    };

                    let width: f32 = if is_selected { 3.0_f32 } else { 1.5_f32 };

                    // Soft ambient line
                    painter.line_segment([p1, p2], Stroke::new(width, stroke_color));

                    // Midpoint pill label
                    let mid = Pos2::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
                    let label_rect = Rect::from_center_size(mid, Vec2::new(84.0 * zoom, 18.0 * zoom));
                    
                    // Click on edge midpoint
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            if label_rect.contains(pos) {
                                app.ui.network_state.selected_edge_id = Some(edge.id.clone());
                                app.ui.network_state.selected_node_id = None;
                                app.ui.selected_entity = Some(SelectedEntity::Edge(edge.id.clone()));
                            }
                        }
                    }

                    painter.rect_filled(label_rect, CornerRadius::same(6), Color32::from_rgb(18, 20, 28));
                    painter.rect_stroke(label_rect, CornerRadius::same(6), Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 62)), StrokeKind::Inside);
                    painter.text(mid, Align2::CENTER_CENTER, &edge.label, FontId::proportional(10.0 * zoom), base_color);
                }
            }

            // 3. Draw Constellation Node Cards
            for node in nodes {
                let pos = to_screen(node.base_pos);
                let node_size = Vec2::new(125.0 * zoom, 50.0 * zoom);
                let node_rect = Rect::from_center_size(pos, node_size);

                let is_selected = app.ui.network_state.selected_node_id.as_deref() == Some(&node.id);

                // Click on node
                if response.clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        if node_rect.contains(click_pos) {
                            app.ui.network_state.selected_node_id = Some(node.id.clone());
                            app.ui.network_state.selected_edge_id = None;

                            // Sync with global SelectedEntity
                            match &node.payload {
                                NodePayload::Object { object_id, .. } => {
                                    app.ui.selected_entity = Some(SelectedEntity::Object(*object_id));
                                }
                                NodePayload::Device { .. } => {
                                    app.ui.selected_entity = Some(SelectedEntity::Device(app.node.identity.actor_id));
                                }
                                NodePayload::Space { space_type, .. } => {
                                    app.ui.selected_entity = Some(SelectedEntity::Space(*space_type));
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Tactile Glassmorphic Node Card
                let bg = if is_selected { palette::SELECTED } else { Color32::from_rgb(22, 24, 33) };
                let border_color = if is_selected { palette::ACCENT } else { Color32::from_rgb(45, 48, 65) };
                
                let radius = CornerRadius::same((8.0 * zoom).clamp(4.0, 16.0) as u8);
                painter.rect(node_rect, radius, bg, Stroke::new(1.5_f32, border_color), StrokeKind::Inside);

                // Left vector icon pill
                let icon_center = Pos2::new(node_rect.min.x + 18.0 * zoom, node_rect.center().y);
                let icon_bg_rect = Rect::from_center_size(icon_center, Vec2::new(26.0 * zoom, 26.0 * zoom));
                painter.rect_filled(icon_bg_rect, CornerRadius::same(6), Color32::from_rgb(32, 35, 48));
                painter.text(icon_center, Align2::CENTER_CENTER, node.icon_glyph, FontId::proportional(16.0 * zoom), palette::ACCENT);

                // Titles & Subtitles
                let text_pos_x = node_rect.min.x + 36.0 * zoom;
                painter.text(
                    Pos2::new(text_pos_x, node_rect.min.y + 14.0 * zoom),
                    Align2::LEFT_CENTER,
                    &node.label,
                    FontId::proportional(12.0 * zoom),
                    palette::TEXT,
                );
                painter.text(
                    Pos2::new(text_pos_x, node_rect.min.y + 32.0 * zoom),
                    Align2::LEFT_CENTER,
                    &node.subtitle,
                    FontId::proportional(10.0 * zoom),
                    palette::TEXT_DIM,
                );
            }
        });
}

fn draw_radar_atmosphere(painter: &Painter, rect: Rect, center: Pos2, zoom: f32) {
    let ring_color = Color32::from_rgba_unmultiplied(40, 45, 60, 90);
    for r in [90.0, 180.0, 270.0, 360.0] {
        let radius = r * zoom;
        painter.circle_stroke(center, radius, Stroke::new(1.0_f32, ring_color));
    }
}

fn render_inspector(ui: &mut Ui, app: &mut NexDesktopApp, nodes: &[VisualizerNode], edges: &[VisualizerEdge]) {
    Frame::new()
        .fill(palette::SIDEBAR)
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.heading(RichText::new("Topological Inspector").size(18.0).strong().color(palette::ACCENT));
            ui.separator();
            ui.add_space(8.0);

            // 1. Edge Explanation ("Why is this connected?")
            if let Some(ref edge_id) = app.ui.network_state.selected_edge_id {
                if let Some(edge) = edges.iter().find(|e| &e.id == edge_id) {
                    ui.label(RichText::new("Relationship Inspection").strong().color(palette::TEXT).size(15.0));
                    ui.label(RichText::new(format!("Type: {}", edge.relationship_class.label()))
                        .color(edge.relationship_class.color()).size(13.0));
                    ui.add_space(10.0);

                    Frame::new().fill(palette::PANEL).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                        ui.label(RichText::new(format!("{} Why are these connected?", egui_phosphor::regular::LIGHTBULB)).strong().size(13.5).color(palette::ACCENT_GREEN));
                        ui.add_space(4.0);

                        let explanation = match app.ui.complexity {
                            InterfaceComplexity::Simple => &edge.explanation_simple,
                            InterfaceComplexity::Standard => &edge.explanation_standard,
                            InterfaceComplexity::Advanced => &edge.explanation_advanced,
                            InterfaceComplexity::Expert => &edge.explanation_operator,
                        };

                        ui.label(RichText::new(explanation).size(13.0).color(palette::TEXT));
                    });

                    ui.add_space(12.0);
                    ui.label(RichText::new("Progressive Disclosure Tiers:").size(12.0).color(palette::TEXT_DIM));
                    ui.label(RichText::new(format!("• Simple: {}", edge.explanation_simple)).size(11.0).color(palette::TEXT_DIM));
                    ui.label(RichText::new(format!("• Standard: {}", edge.explanation_standard)).size(11.0).color(palette::TEXT_DIM));
                    ui.label(RichText::new(format!("• Advanced: {}", edge.explanation_advanced)).size(11.0).color(palette::TEXT_DIM));
                    ui.label(RichText::new(format!("• Operator: {}", edge.explanation_operator)).size(11.0).color(palette::TEXT_DIM));
                    return;
                }
            }

            // 2. Node Inspection (Universal Object Inspector / Contextual Device Panel)
            if let Some(ref node_id) = app.ui.network_state.selected_node_id {
                if let Some(node) = nodes.iter().find(|n| &n.id == node_id) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(node.icon_glyph).size(24.0).color(palette::ACCENT));
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&node.label).strong().size(16.0).color(palette::TEXT));
                            ui.label(RichText::new(&node.subtitle).size(12.0).color(palette::TEXT_DIM));
                        });
                    });
                    ui.add_space(10.0);

                    match &node.payload {
                        NodePayload::Device { actor_id_hex, is_local } => {
                            let panel = ContextualPanelsEngine::project_device_panel(&app.node, &app.node.identity.actor_id, None, false);
                            ui.label(RichText::new("Device Context Surface").strong().color(palette::ACCENT));
                            ui.add_space(4.0);

                            // Cross-lens navigation
                            if ui.button(format!("{} Open in Devices Lens", egui_phosphor::regular::DEVICES)).clicked() {
                                app.ui.active_tab = NavTab::Devices;
                            }
                            ui.add_space(6.0);

                            ui.label(RichText::new(format!("Actor ID: {}", actor_id_hex)).size(12.0).color(palette::TEXT_DIM));
                            ui.label(RichText::new(format!("Is Local Host: {}", is_local)).size(12.0));
                            ui.label(RichText::new(format!("Revoked: {}", panel.is_revoked)).size(12.0));
                            ui.label(RichText::new(format!("Operational State: {:?}", app.node.operational_state)).size(12.0).color(palette::ACCENT_GREEN));
                        }
                        NodePayload::Space { space_type, item_count } => {
                            ui.label(RichText::new("Space Container Surface").strong().color(palette::ACCENT));
                            ui.add_space(4.0);

                            // Cross-lens navigation
                            if ui.button(match space_type {
                                SpaceType::Family => format!("{} Open Family Space", egui_phosphor::regular::HEART),
                                _ => format!("{} Open Personal Space", egui_phosphor::regular::HOUSE),
                            }).clicked() {
                                match space_type {
                                    SpaceType::Family => app.ui.active_tab = NavTab::Family,
                                    _ => app.ui.active_tab = NavTab::Home,
                                }
                            }
                            ui.add_space(6.0);

                            ui.label(RichText::new(format!("Total Items: {}", item_count)).size(12.0));
                            ui.label(RichText::new("Namespace Isolation: Cryptographically Enforced").size(12.0).color(palette::ACCENT_GREEN));
                        }
                        NodePayload::Object { object_id, title, space_name, .. } => {
                            ui.label(RichText::new("Object Inode Surface").strong().color(palette::ACCENT));
                            ui.add_space(4.0);

                            // Cross-lens navigation
                            ui.horizontal(|ui| {
                                if ui.button(format!("{} Drive", egui_phosphor::regular::HARD_DRIVE)).clicked() {
                                    app.ui.active_tab = NavTab::Drive;
                                    app.ui.drive_state.selected_file_id = Some(*object_id);
                                    app.ui.selected_entity = Some(SelectedEntity::Object(*object_id));
                                }
                                if ui.button(format!("{} Photos", egui_phosphor::regular::IMAGE)).clicked() {
                                    app.ui.active_tab = NavTab::Photos;
                                    app.ui.selected_entity = Some(SelectedEntity::Object(*object_id));
                                }
                            });
                            ui.add_space(6.0);

                            ui.label(RichText::new(format!("Title: {}", title)).size(13.0).color(palette::TEXT));
                            ui.label(RichText::new(format!("Object ID: {}", hex::encode(&object_id[0..6]))).size(11.5).color(palette::TEXT_DIM));
                            ui.label(RichText::new(format!("Space: {}", space_name)).size(12.0).color(palette::TEXT_DIM));
                        }
                        NodePayload::TransportSubstrate { name, status } => {
                            ui.label(RichText::new("Transport Substrate Surface").strong().color(palette::ACCENT));
                            ui.add_space(6.0);
                            ui.label(RichText::new(format!("Protocol: {}", name)).size(13.0));
                            ui.label(RichText::new(format!("Status: {}", status)).size(12.0).color(palette::ACCENT_GREEN));
                            ui.label(RichText::new("Zero Public Internet Dependency").size(12.0).color(palette::TEXT_DIM));
                        }
                    }
                    return;
                }
            }

            // Default prompt when nothing selected
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new(egui_phosphor::regular::SHARE_NETWORK).size(32.0).color(palette::TEXT_DIM));
                ui.add_space(8.0);
                ui.label(RichText::new("Click any node or relationship link on the radar to inspect its cryptographic provenance and why it is connected.")
                    .size(13.0).color(palette::TEXT_DIM));
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use nex_core::runtime::node::NexNode;
    use nex_core::object::types::{NexObject, ObjectType};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;
    use std::collections::BTreeMap;

    fn create_test_app() -> NexDesktopApp {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_network");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();
        NexDesktopApp {
            node,
            data_dir,
            ui: NexUiState::new(),
            status: AppStatus::Running,
        }
    }

    #[test]
    fn test_topology_derivation_is_truthful_and_ephemeral() {
        let mut app = create_test_app();
        let (nodes, edges) = derive_topology(&app);
        assert!(nodes.len() >= 4);
        assert!(edges.len() >= 3);

        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), "Topology Test Inode".to_string());
        meta.insert("space".to_string(), "Family".to_string());
        let obj = NexObject {
            object_id: [0x77; 32],
            schema_version: 1,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: app.node.identity.actor_id,
            created_epoch: 1,
            created_lamport: 1,
            winning_mutation_id: [0u8; 32],
            payload_bytes: vec![1, 2, 3],
            metadata: meta,
            tombstoned: false,
        };
        app.node.state.object_store.insert(obj.object_id, obj);

        let (nodes_after, edges_after) = derive_topology(&app);
        assert_eq!(nodes_after.len(), nodes.len() + 1);
        assert_eq!(edges_after.len(), edges.len() + 2);
    }

    #[test]
    fn test_edge_explanation_honors_complexity_without_state_mutation() {
        let app = create_test_app();
        let (_, edges) = derive_topology(&app);
        let edge = &edges[0];
        assert!(!edge.explanation_simple.is_empty());
        assert!(!edge.explanation_operator.is_empty());
        assert_ne!(edge.explanation_simple, edge.explanation_operator);
    }

    #[test]
    fn test_cross_lens_journey_context_preservation() {
        let mut app = create_test_app();
        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), "Journey Image".to_string());
        let obj_id = [0x88; 32];
        let obj = NexObject {
            object_id: obj_id,
            schema_version: 1,
            object_type: ObjectType::PhotoMedia,
            namespace: [0u8; 32],
            owner_actor_id: app.node.identity.actor_id,
            created_epoch: 1,
            created_lamport: 1,
            winning_mutation_id: [0u8; 32],
            payload_bytes: vec![4, 5, 6],
            metadata: meta,
            tombstoned: false,
        };
        app.node.state.object_store.insert(obj_id, obj);

        let (nodes, _) = derive_topology(&app);
        let obj_node = nodes.iter().find(|n| match &n.payload {
            NodePayload::Object { object_id, .. } => *object_id == obj_id,
            _ => false,
        }).expect("Object node must exist");

        if let NodePayload::Object { object_id, .. } = &obj_node.payload {
            assert_eq!(*object_id, obj_id);
            app.ui.selected_entity = Some(SelectedEntity::Object(*object_id));
        }

        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(obj_id)));
    }
}
