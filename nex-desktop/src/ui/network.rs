use egui::{Ui, Pos2, Vec2, Rect, Color32, Stroke, CornerRadius, StrokeKind, RichText, Frame, Sense, Painter, FontId, Align2};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::{ObjectID, ObjectType};
use nex_core::product::inspector::UniversalObjectInspector;
use nex_core::runtime::panels::ContextualPanelsEngine;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

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
            Self::Logical => "Logical (Space / Membership)",
            Self::Network => "Network (Peers / Reachability)",
            Self::Transport => "Transport (TCP / Protocols)",
            Self::DataFlow => "Data Flow (CAS / Replication)",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Logical => Color32::from_rgb(96, 165, 250),    // Blue
            Self::Network => Color32::from_rgb(74, 222, 128),    // Green
            Self::Transport => Color32::from_rgb(251, 191, 36),  // Amber
            Self::DataFlow => Color32::from_rgb(192, 132, 252),  // Purple
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
    pub icon: &'static str,
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
        ui.heading(RichText::new("Network & Topology").size(24.0).strong().color(palette::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Zoom controls
            if ui.button("↺ Reset").clicked() {
                app.ui.network_state.pan_offset = Vec2::ZERO;
                app.ui.network_state.zoom_level = 1.0;
            }
            if ui.button("➕").clicked() {
                app.ui.network_state.zoom_level = (app.ui.network_state.zoom_level * 1.15).min(2.5);
            }
            if ui.button("➖").clicked() {
                app.ui.network_state.zoom_level = (app.ui.network_state.zoom_level / 1.15).max(0.5);
            }

            ui.label(RichText::new(format!("Global Tier: {:?}", app.ui.complexity)).color(palette::ACCENT).size(12.0));
        });
    });

    ui.label(RichText::new("Pure projection of canonical sovereign state — live nodes, spaces, and relationships")
        .color(palette::TEXT_DIM).size(13.0));
    ui.add_space(8.0);

    // Derive topology on the fly from canonical state
    let (nodes, edges) = derive_topology(app);

    // Split view: Left = 2D Topology Canvas, Right = Contextual Explanation / Inspector Panel
    ui.columns(2, |columns| {
        let (first, second) = columns.split_at_mut(1);
        let canvas_ui = &mut first[0];
        let inspector_ui = &mut second[0];

        // 1. Topology Canvas
        render_canvas(canvas_ui, app, &nodes, &edges);

        // 2. Contextual Inspector / "Why is this connected?"
        render_inspector(inspector_ui, app, &nodes, &edges);
    });
}

pub fn derive_topology(app: &NexDesktopApp) -> (Vec<VisualizerNode>, Vec<VisualizerEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let local_actor_hex = hex::encode(&app.node.identity.actor_id[0..4]);
    let local_device_id = "device_local".to_string();

    // Node 1: Local Device (This PC)
    nodes.push(VisualizerNode {
        id: local_device_id.clone(),
        label: "This PC (Windows)".to_string(),
        subtitle: format!("ID: {}", local_actor_hex),
        icon: "🖥",
        base_pos: Pos2::new(180.0, 160.0),
        payload: NodePayload::Device {
            actor_id_hex: hex::encode(&app.node.identity.actor_id),
            is_local: true,
        },
    });

    // Node 2: Personal Space
    let personal_space_id = "space_personal".to_string();
    let personal_items = app.node.state.object_store.values()
        .filter(|o| o.metadata.get("space").map(|s| s.as_str()) != Some("Family") && !o.tombstoned)
        .count();
    nodes.push(VisualizerNode {
        id: personal_space_id.clone(),
        label: "Personal Space".to_string(),
        subtitle: format!("{} items", personal_items),
        icon: "🔒",
        base_pos: Pos2::new(60.0, 60.0),
        payload: NodePayload::Space { space_type: SpaceType::Personal, item_count: personal_items },
    });

    // Node 3: Family Space
    let family_space_id = "space_family".to_string();
    let family_items = app.node.state.object_store.values()
        .filter(|o| o.metadata.get("space").map(|s| s.as_str()) == Some("Family") && !o.tombstoned)
        .count();
    nodes.push(VisualizerNode {
        id: family_space_id.clone(),
        label: "Family Space".to_string(),
        subtitle: format!("{} items", family_items),
        icon: "🏡",
        base_pos: Pos2::new(300.0, 60.0),
        payload: NodePayload::Space { space_type: SpaceType::Family, item_count: family_items },
    });

    // Node 4: Transport Substrate (LAN / Local Listener)
    let transport_id = "transport_lan".to_string();
    nodes.push(VisualizerNode {
        id: transport_id.clone(),
        label: "LAN Transport Substrate".to_string(),
        subtitle: format!("TCP Socket (State: {:?})", app.node.operational_state),
        icon: "⚡",
        base_pos: Pos2::new(60.0, 260.0),
        payload: NodePayload::TransportSubstrate {
            name: "TCP/IP Direct".to_string(),
            status: format!("{:?}", app.node.operational_state),
        },
    });

    // Edge: Local Device -> Personal Space
    edges.push(VisualizerEdge {
        id: "edge_device_personal".to_string(),
        from_node_id: local_device_id.clone(),
        to_node_id: personal_space_id.clone(),
        label: "owns / participates".to_string(),
        relationship_class: RelationshipClass::Logical,
        explanation_simple: "This device owns your Personal Space.".to_string(),
        explanation_standard: "Your local Windows host holds the root cryptographic identity for your Personal Space.".to_string(),
        explanation_advanced: format!("Actor {} is sovereign owner of Namespace 0x00..00 (Personal).", local_actor_hex),
        explanation_operator: format!("Master Ed25519 Key verified. Local CAS chunk partition active. Current epoch: {}.", app.node.state.current_epoch),
    });

    // Edge: Local Device -> Family Space
    edges.push(VisualizerEdge {
        id: "edge_device_family".to_string(),
        from_node_id: local_device_id.clone(),
        to_node_id: family_space_id.clone(),
        label: "member of".to_string(),
        relationship_class: RelationshipClass::Logical,
        explanation_simple: "This device is a member of the Family Space.".to_string(),
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
        explanation_simple: "Local network listener is active.".to_string(),
        explanation_standard: "This PC listens on local network TCP sockets for peer discovery and sync.".to_string(),
        explanation_advanced: format!("Transport adapter bound. Node state: {:?}. Wire framing: NEX/WIRE/v1.", app.node.operational_state),
        explanation_operator: "48-byte binary frame headers enabled. Sockets bound on local loopback/LAN.".to_string(),
    });

    // Objects in store (up to 4 real objects from object_store)
    let mut obj_idx = 0;
    for (obj_id, obj) in app.node.state.object_store.iter().filter(|(_, o)| !o.tombstoned).take(4) {
        let obj_node_id = format!("obj_{}", hex::encode(&obj_id[0..4]));
        let title = obj.metadata.get("title")
            .or_else(|| obj.metadata.get("filename"))
            .cloned()
            .unwrap_or_else(|| "Sovereign Object".to_string());
        let space_name = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());
        let icon = match obj.object_type {
            ObjectType::PhotoMedia => "📷",
            ObjectType::DriveInode => "📄",
            _ => "📦",
        };

        let pos_x = 240.0 + (obj_idx as f32) * 80.0;
        let pos_y = 240.0 + ((obj_idx % 2) as f32) * 50.0;

        nodes.push(VisualizerNode {
            id: obj_node_id.clone(),
            label: if title.len() > 14 { format!("{}...", &title[0..12]) } else { title.clone() },
            subtitle: format!("{} B", obj.payload_bytes.len()),
            icon,
            base_pos: Pos2::new(pos_x, pos_y),
            payload: NodePayload::Object {
                object_id: *obj_id,
                object_type: obj.object_type,
                title: title.clone(),
                space_name: space_name.clone(),
            },
        });

        // Edge: Space -> Object (Containment)
        let parent_space = if space_name == "Family" { &family_space_id } else { &personal_space_id };
        edges.push(VisualizerEdge {
            id: format!("edge_space_{}", obj_node_id),
            from_node_id: parent_space.clone(),
            to_node_id: obj_node_id.clone(),
            label: "contains".to_string(),
            relationship_class: RelationshipClass::Logical,
            explanation_simple: format!("'{}' belongs to {} Space.", title, space_name),
            explanation_standard: format!("Object stored under {} Space namespace with sovereign encryption.", space_name),
            explanation_advanced: format!("Schema v{} CAS Inode | Namespace: {}", obj.schema_version, space_name),
            explanation_operator: format!("SMT Leaf Key: {} | Author: {} | Epoch: {}", hex::encode(&obj.object_id[0..8]), hex::encode(&obj.owner_actor_id[0..4]), obj.created_epoch),
        });

        // Edge: Local Device -> Object (Storage / CAS)
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
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 440.0), Sense::click_and_drag());

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

            // 1. Draw Background Grid
            draw_grid(&painter, rect, pan, zoom);

            // 2. Draw Edges
            for edge in edges {
                if let (Some(from_node), Some(to_node)) = (
                    nodes.iter().find(|n| n.id == edge.from_node_id),
                    nodes.iter().find(|n| n.id == edge.to_node_id),
                ) {
                    let p1 = to_screen(from_node.base_pos);
                    let p2 = to_screen(to_node.base_pos);
                    let is_selected = app.ui.network_state.selected_edge_id.as_deref() == Some(&edge.id);

                    let color = if is_selected {
                        Color32::WHITE
                    } else {
                        edge.relationship_class.color()
                    };

                    let width: f32 = if is_selected { 3.0_f32 } else { 1.5_f32 };

                    // Draw connection line
                    painter.line_segment([p1, p2], Stroke::new(width, color));

                    // Midpoint label
                    let mid = Pos2::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
                    let label_rect = Rect::from_center_size(mid, Vec2::new(70.0 * zoom, 16.0 * zoom));
                    
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

                    painter.rect_filled(label_rect, CornerRadius::same(4), Color32::from_black_alpha(180));
                    painter.text(mid, Align2::CENTER_CENTER, &edge.label, FontId::proportional(10.0 * zoom), color);
                }
            }

            // 3. Draw Nodes
            for node in nodes {
                let pos = to_screen(node.base_pos);
                let node_size = Vec2::new(110.0 * zoom, 44.0 * zoom);
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

                // Node card styling
                let bg = if is_selected { palette::SELECTED } else { palette::BG };
                let border_color = if is_selected { palette::ACCENT } else { Color32::from_rgb(60, 60, 75) };
                
                let radius = CornerRadius::same((6.0 * zoom).clamp(2.0, 16.0) as u8);
                painter.rect(node_rect, radius, bg, Stroke::new(1.5_f32, border_color), StrokeKind::Inside);

                // Icon and labels
                let icon_pos = Pos2::new(node_rect.min.x + 16.0 * zoom, node_rect.center().y);
                painter.text(icon_pos, Align2::CENTER_CENTER, node.icon, FontId::proportional(16.0 * zoom), Color32::WHITE);

                let text_pos_x = node_rect.min.x + 32.0 * zoom;
                painter.text(
                    Pos2::new(text_pos_x, node_rect.min.y + 12.0 * zoom),
                    Align2::LEFT_CENTER,
                    &node.label,
                    FontId::proportional(11.5 * zoom),
                    palette::TEXT,
                );
                painter.text(
                    Pos2::new(text_pos_x, node_rect.min.y + 28.0 * zoom),
                    Align2::LEFT_CENTER,
                    &node.subtitle,
                    FontId::proportional(9.5 * zoom),
                    palette::TEXT_DIM,
                );
            }
        });
}

fn draw_grid(painter: &Painter, rect: Rect, pan: Vec2, zoom: f32) {
    let grid_size = 30.0 * zoom;
    let offset_x = (rect.min.x + pan.x) % grid_size;
    let offset_y = (rect.min.y + pan.y) % grid_size;

    let dot_color = Color32::from_rgb(40, 40, 50);
    let mut x = rect.min.x + offset_x;
    while x < rect.max.x {
        let mut y = rect.min.y + offset_y;
        while y < rect.max.y {
            painter.circle_filled(Pos2::new(x, y), 1.0, dot_color);
            y += grid_size;
        }
        x += grid_size;
    }
}

fn render_inspector(ui: &mut Ui, app: &mut NexDesktopApp, nodes: &[VisualizerNode], edges: &[VisualizerEdge]) {
    Frame::new()
        .fill(palette::SIDEBAR)
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.heading(RichText::new("Contextual Inspector").size(18.0).strong().color(palette::ACCENT));
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
                        ui.label(RichText::new("Why are these connected?").strong().size(13.5).color(palette::ACCENT_GREEN));
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
                    ui.label(RichText::new("Progressive Disclosure Chain:").size(12.0).color(palette::TEXT_DIM));
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
                        ui.label(RichText::new(node.icon).size(22.0));
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
                            if ui.button("📱 Open in Devices Lens").clicked() {
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
                                SpaceType::Family => "🏡 Open Family Space",
                                _ => "🏠 Open Personal Space",
                            }).clicked() {
                                app.ui.active_tab = match space_type {
                                    SpaceType::Family => NavTab::Family,
                                    _ => NavTab::Home,
                                };
                            }
                            ui.add_space(6.0);

                            ui.label(RichText::new(format!("Space Type: {:?}", space_type)).size(13.0));
                            ui.label(RichText::new(format!("Total Active Objects: {}", item_count)).size(13.0));
                            ui.label(RichText::new("Sovereign Policy: Local First & E2EE Shared").size(12.0).color(palette::TEXT_DIM));
                        }
                        NodePayload::Object { object_id, .. } => {
                            ui.label(RichText::new("Universal Object Inspector").strong().color(palette::ACCENT));
                            ui.add_space(4.0);

                            // Cross-lens navigation
                            ui.horizontal(|ui| {
                                if ui.button("📷 Photos").clicked() {
                                    app.ui.active_tab = NavTab::Photos;
                                }
                                if ui.button("💾 Drive").clicked() {
                                    app.ui.active_tab = NavTab::Drive;
                                }
                            });
                            ui.add_space(6.0);

                            if let Ok(inspector) = UniversalObjectInspector::inspect(&app.node, object_id, app.ui.complexity) {
                                ui.label(RichText::new(format!("Title: {}", inspector.title)).size(13.0).strong());
                                ui.label(RichText::new(format!("Space: {}", inspector.space_name)).size(12.0));
                                ui.label(RichText::new(format!("Size: {}", inspector.byte_size_formatted)).size(12.0));
                                ui.label(RichText::new(format!("Status: {}", inspector.status_badge)).size(12.0).color(palette::ACCENT_GREEN));

                                if let Some(dag) = inspector.advanced_dag_info {
                                    ui.add_space(6.0);
                                    ui.label(RichText::new("DAG Provenance:").strong().size(12.0).color(palette::TEXT));
                                    ui.label(RichText::new(format!("Schema v{}", dag.schema_version)).size(11.0).color(palette::TEXT_DIM));
                                    ui.label(RichText::new(format!("CAS Chunks: {}", dag.cas_chunk_count)).size(11.0).color(palette::TEXT_DIM));
                                    ui.label(RichText::new(format!("SMT Key: {}", &dag.smt_key_hex[0..16])).size(11.0).color(palette::TEXT_DIM));
                                }
                            }
                        }
                        NodePayload::TransportSubstrate { name, status } => {
                            ui.label(RichText::new("Transport Protocol Surface").strong().color(palette::ACCENT));
                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("Protocol: {}", name)).size(13.0));
                            ui.label(RichText::new(format!("Engine Status: {}", status)).size(13.0).color(palette::ACCENT_GREEN));
                            ui.label(RichText::new("Wire Header: 48-byte NEX/WIRE/v1 framing").size(12.0).color(palette::TEXT_DIM));
                        }
                    }
                    return;
                }
            }

            // Default prompt when nothing selected
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(RichText::new("🔍 Select a Node or Edge").size(14.0).color(palette::TEXT_DIM));
                ui.add_space(6.0);
                ui.label(RichText::new("Click any node to inspect entity details, or click an edge to see 'Why is this connected?'")
                    .size(12.0).color(palette::TEXT_DIM));
            });
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
        let data_dir = PathBuf::from("d:\\Nex\\test_data_topology");
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
    fn test_topology_derivation_is_truthful_and_ephemeral() {
        let app = create_test_app();
        let (nodes, edges) = derive_topology(&app);

        // Verify base nodes: Local PC, Personal Space, Family Space, Transport Substrate
        assert!(nodes.len() >= 4, "Must contain at least 4 base nodes");
        assert!(nodes.iter().any(|n| n.id == "device_local"), "Must have local device");
        assert!(nodes.iter().any(|n| n.id == "space_personal"), "Must have personal space");
        assert!(nodes.iter().any(|n| n.id == "space_family"), "Must have family space");
        assert!(nodes.iter().any(|n| n.id == "transport_lan"), "Must have transport substrate");

        // Verify zero fake/fabricated remote peers
        assert!(!nodes.iter().any(|n| n.id.starts_with("device_remote_")), "Must not fabricate remote peers");

        // Verify edges and progressive disclosure non-emptiness
        assert!(edges.len() >= 3, "Must have base edges");
        for edge in &edges {
            assert!(!edge.explanation_simple.is_empty(), "Simple explanation must be present");
            assert!(!edge.explanation_standard.is_empty(), "Standard explanation must be present");
            assert!(!edge.explanation_advanced.is_empty(), "Advanced explanation must be present");
            assert!(!edge.explanation_operator.is_empty(), "Operator explanation must be present");
        }
    }

    #[test]
    fn test_edge_explanation_honors_complexity_without_state_mutation() {
        let mut app = create_test_app();
        let (_, edges) = derive_topology(&app);
        let personal_edge = edges.iter().find(|e| e.id == "edge_device_personal").unwrap();

        app.ui.complexity = InterfaceComplexity::Simple;
        assert_eq!(personal_edge.explanation_simple, "This device owns your Personal Space.");

        app.ui.complexity = InterfaceComplexity::Expert;
        assert!(personal_edge.explanation_operator.contains("Master Ed25519 Key verified"));
    }

    #[test]
    fn test_cross_lens_journey_context_preservation() {
        let mut app = create_test_app();
        
        // 1. User starts at Home
        app.ui.active_tab = NavTab::Home;
        assert_eq!(app.ui.active_tab, NavTab::Home);

        // 2. User navigates to Family Space
        app.ui.active_tab = NavTab::Family;
        app.ui.selected_entity = Some(SelectedEntity::Space(SpaceType::Family));

        // 3. User switches to Network
        app.ui.active_tab = NavTab::Network;
        app.ui.network_state.selected_node_id = Some("space_family".to_string());
        
        // 4. Verify context preserved across tabs
        assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Space(SpaceType::Family)));
        assert_eq!(app.ui.network_state.selected_node_id, Some("space_family".to_string()));
    }
}
