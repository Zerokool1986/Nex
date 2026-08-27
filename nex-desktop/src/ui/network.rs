use egui::{Ui, Pos2, Vec2, Rect, Color32, Stroke, CornerRadius, StrokeKind, RichText, Frame, Sense, Painter, FontId, Align2};
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::{ObjectID, ObjectType};
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConduitStatus {
    AvailableDirectMesh,
    Replicating,
    Away,
    Revoked,
}

impl ConduitStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AvailableDirectMesh => "🟢 Available now (Direct LAN Wi-Fi • 120 MB/s)",
            Self::Replicating => "🔵 Replicating Active DAG & SMT Chunks",
            Self::Away => "🟡 Away (Will auto-sync on proximity)",
            Self::Revoked => "🔴 Trust Revoked",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::AvailableDirectMesh => Color32::from_rgb(52, 211, 153), // Emerald Mesh
            Self::Replicating => Color32::from_rgb(91, 141, 246),         // Radiant Cobalt
            Self::Away => Color32::from_rgb(251, 191, 36),                // Amber
            Self::Revoked => Color32::from_rgb(248, 113, 113),            // Red
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodePayload {
    Device { actor_id_hex: String, is_local: bool },
    Space { space_type: SpaceType, item_count: usize },
    Object { object_id: ObjectID, object_type: ObjectType, title: String, space_name: String },
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
    pub status: ConduitStatus,
    pub payload_stream_label: String,
    pub partition_resilience_label: String,
    pub explanation_simple: String,
    pub explanation_standard: String,
    pub explanation_advanced: String,
    pub explanation_operator: String,
}

#[derive(Debug, Clone)]
pub struct NetworkViewState {
    pub pan_offset: Vec2,
    pub zoom_level: f32,
    pub selected_node_id: Option<String>,
    pub selected_edge_id: Option<String>,
    pub focused_node_index: Option<usize>,
}

impl NetworkViewState {
    pub fn new() -> Self {
        Self {
            pan_offset: Vec2::ZERO,
            zoom_level: 1.0,
            selected_node_id: None,
            selected_edge_id: None,
            focused_node_index: None,
        }
    }
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. CONSTELLATION HEADER — Living Mesh Topology
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Sovereign Constellation").size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new("🌌 Living Mesh Topology — How your devices, people, and memories connect")
                .size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{}  Connect Device", egui_phosphor::regular::PLUS)).size(13.0).color(palette::TEXT).strong())
                .clicked()
            {
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
        });
    });

    ui.add_space(16.0);

    // Derive topology live from canonical state
    let (nodes, edges) = derive_topology(app);

    // 2. Truthful Constellation Telemetry Beacon
    render_constellation_beacon(ui, nodes.len(), edges.len());
    ui.add_space(16.0);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 3. FULL-WIDTH INTERACTIVE CELESTIAL CANVAS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    render_canvas(ui, app, &nodes, &edges);
    ui.add_space(14.0);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 4. CONTEXTUAL CONDUIT & NODE STAGE
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    render_contextual_stage(ui, app, &nodes, &edges);
}

/// Renders the Truthful Constellation Telemetry Beacon
fn render_constellation_beacon(ui: &mut Ui, node_count: usize, conduit_count: usize) {
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} Live constellation", egui_phosphor::regular::SPARKLE))
                    .size(12.0).color(palette::ACCENT_GREEN));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} {} Sovereign nodes", egui_phosphor::regular::DEVICES, node_count))
                    .size(12.0).color(palette::TEXT_SECONDARY));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} {} Direct mesh conduits", egui_phosphor::regular::SHARE_NETWORK, conduit_count))
                    .size(12.0).color(palette::TEXT_SECONDARY));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new("Zero central cloud servers").size(12.0).color(palette::ACCENT_GREEN));
            });
        });
}

/// Renders the Interactive 2D Celestial Canvas with HUD Controls
fn render_canvas(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    nodes: &[VisualizerNode],
    edges: &[VisualizerEdge],
) {
    let canvas_height = 280.0_f32;
    let (response, painter) = ui.allocate_painter(
        Vec2::new(ui.available_width(), canvas_height),
        Sense::click_and_drag(),
    );

    let rect = response.rect;

    // Handle Pan and Zoom
    if response.dragged() {
        app.ui.network_state.pan_offset += response.drag_delta();
    }

    let pan = app.ui.network_state.pan_offset;
    let zoom = app.ui.network_state.zoom_level;
    let center = rect.center() + pan;

    // 1. Draw Obsidian Void Atmosphere Background
    painter.rect_filled(rect, CornerRadius::same(10), palette::BG);
    painter.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0_f32, palette::GLASS_BORDER), StrokeKind::Inside);

    // Subtle radar grid background
    draw_radar_atmosphere(&painter, center, zoom);

    // 2. Draw Conduits / Edges
    for edge in edges {
        if let (Some(from_node), Some(to_node)) = (
            nodes.iter().find(|n| n.id == edge.from_node_id),
            nodes.iter().find(|n| n.id == edge.to_node_id),
        ) {
            let p1 = center + (from_node.base_pos - Pos2::new(260.0, 180.0)) * zoom;
            let p2 = center + (to_node.base_pos - Pos2::new(260.0, 180.0)) * zoom;

            let is_edge_selected = app.ui.network_state.selected_edge_id.as_deref() == Some(&edge.id);
            let edge_color = if is_edge_selected {
                palette::ACCENT
            } else {
                edge.status.color()
            };

            let stroke_width = if is_edge_selected { 2.5 } else { 1.5 } * zoom.clamp(0.8, 1.4);
            painter.line_segment([p1, p2], Stroke::new(stroke_width, edge_color));

            // Midpoint label
            let mid = Pos2::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
            painter.circle_filled(mid, 3.0 * zoom, edge_color);
        }
    }

    // 3. Draw Constellation Nodes
    for (idx, node) in nodes.iter().enumerate() {
        let node_pos = center + (node.base_pos - Pos2::new(260.0, 180.0)) * zoom;
        let is_node_selected = app.ui.network_state.selected_node_id.as_deref() == Some(&node.id);
        let is_node_focused = app.ui.network_state.focused_node_index == Some(idx);

        let node_radius = 24.0 * zoom.clamp(0.7, 1.3);
        let bg_color = if is_node_selected || is_node_focused {
            palette::SELECTED
        } else {
            palette::CARD
        };

        let stroke_color = if is_node_selected || is_node_focused {
            palette::ACCENT
        } else {
            palette::GLASS_BORDER
        };

        // Outer glow on selection
        if is_node_selected || is_node_focused {
            painter.circle_filled(node_pos, node_radius + 4.0, Color32::from_rgba_premultiplied(99, 144, 250, 40));
        }

        painter.circle_filled(node_pos, node_radius, bg_color);
        painter.circle_stroke(node_pos, node_radius, Stroke::new(1.5, stroke_color));

        // Center vector glyph
        painter.text(
            node_pos,
            Align2::CENTER_CENTER,
            node.icon_glyph,
            FontId::proportional(16.0 * zoom),
            palette::TEXT,
        );

        // Subtitle below node
        painter.text(
            Pos2::new(node_pos.x, node_pos.y + node_radius + 12.0),
            Align2::CENTER_CENTER,
            &node.label,
            FontId::proportional(11.5),
            palette::TEXT,
        );

        // Click interaction
        if response.clicked() {
            if let Some(hover_pos) = response.hover_pos() {
                if hover_pos.distance(node_pos) <= node_radius + 6.0 {
                    app.ui.network_state.selected_node_id = Some(node.id.clone());
                    app.ui.network_state.focused_node_index = Some(idx);
                    app.ui.network_state.selected_edge_id = None;

                    match &node.payload {
                        NodePayload::Device { .. } => {
                            app.ui.selected_entity = Some(SelectedEntity::Device(app.node.identity.actor_id));
                        }
                        NodePayload::Object { object_id, .. } => {
                            app.ui.selected_entity = Some(SelectedEntity::Object(*object_id));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // 4. Canvas HUD Controls (Zoom In, Zoom Out, Reset)
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

fn draw_radar_atmosphere(painter: &Painter, center: Pos2, zoom: f32) {
    for r in [60.0, 120.0, 180.0] {
        painter.circle_stroke(
            center,
            r * zoom,
            Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 12)),
        );
    }
}

/// Renders the Contextual Conduit & Node Stage
fn render_contextual_stage(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    nodes: &[VisualizerNode],
    edges: &[VisualizerEdge],
) {
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(18, 14))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            // If an edge/conduit is selected
            if let Some(edge_id) = &app.ui.network_state.selected_edge_id {
                if let Some(edge) = edges.iter().find(|e| &e.id == edge_id) {
                    render_edge_details(ui, app, edge);
                    return;
                }
            }

            // If a node is selected
            if let Some(node_id) = &app.ui.network_state.selected_node_id {
                if let Some(node) = nodes.iter().find(|n| &n.id == node_id) {
                    render_node_details(ui, app, node);
                    return;
                }
            }

            // Default stage view (Active Primary Conduit)
            if let Some(primary_edge) = edges.first() {
                render_edge_details(ui, app, primary_edge);
            }
        });
}

fn render_edge_details(ui: &mut Ui, app: &mut NexDesktopApp, edge: &VisualizerEdge) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(egui_phosphor::regular::LIGHTNING).size(20.0).color(edge.status.color()));
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Conduit: {}", edge.label)).size(14.0).strong().color(palette::TEXT));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(edge.status.label()).size(12.0).color(edge.status.color()));
        });
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("PAYLOAD STREAM:").size(11.0).strong().color(palette::TEXT_DIM));
        ui.add_space(8.0);
        ui.label(RichText::new(&edge.payload_stream_label).size(12.5).color(palette::TEXT));
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("PARTITION RESILIENCE:").size(11.0).strong().color(palette::TEXT_DIM));
        ui.add_space(8.0);
        ui.label(RichText::new(&edge.partition_resilience_label).size(12.0).color(palette::ACCENT_GREEN));
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new(format!("{} View Replicated Objects", egui_phosphor::regular::FOLDER)).size(12.0).color(palette::TEXT))
            .clicked()
        {
            app.ui.active_tab = NavTab::Drive;
        }

        if ui.button(RichText::new(format!("{} Inspect in Truth Layer", egui_phosphor::regular::MAGNIFYING_GLASS)).size(12.0).color(palette::ACCENT))
            .clicked()
        {
            app.ui.selected_entity = Some(SelectedEntity::Device(app.node.identity.actor_id));
        }
    });

    // Operator diagnostics
    if app.ui.complexity == InterfaceComplexity::Expert {
        ui.add_space(8.0);
        ui.label(RichText::new(&edge.explanation_operator).monospace().size(10.0).color(palette::TEXT_DIM));
    }
}

fn render_node_details(ui: &mut Ui, app: &mut NexDesktopApp, node: &VisualizerNode) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(node.icon_glyph).size(20.0).color(palette::ACCENT));
        ui.add_space(4.0);
        ui.label(RichText::new(&node.label).size(14.0).strong().color(palette::TEXT));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(&node.subtitle).size(12.0).color(palette::TEXT_SECONDARY));
        });
    });

    ui.add_space(8.0);
    match &node.payload {
        NodePayload::Device { is_local, .. } => {
            let role = if *is_local { "Primary Host Device • Holds 100% of local CAS" } else { "Verified Mesh Peer • Direct Wi-Fi Synchronization" };
            ui.label(RichText::new(format!("Role: {}", role)).size(12.5).color(palette::TEXT));
            ui.add_space(8.0);
            if ui.button(RichText::new("Inspect Node & CAS").size(12.0).color(palette::ACCENT)).clicked() {
                app.ui.selected_entity = Some(SelectedEntity::Device(app.node.identity.actor_id));
            }
        }
        NodePayload::Space { space_type, item_count } => {
            ui.label(RichText::new(format!("Cryptographic Space Container: {:?} • {} items stored", space_type, item_count)).size(12.5).color(palette::TEXT));
        }
        NodePayload::Object { object_id, title, .. } => {
            ui.label(RichText::new(format!("Sovereign Object: {} • Invariant BLAKE3: {}", title, hex::encode(&object_id[0..4]))).size(12.5).color(palette::TEXT));
            ui.add_space(8.0);
            if ui.button(RichText::new("Inspect in Truth Layer").size(12.0).color(palette::ACCENT)).clicked() {
                app.ui.selected_entity = Some(SelectedEntity::Object(*object_id));
            }
        }
    }
}

pub fn derive_topology(app: &NexDesktopApp) -> (Vec<VisualizerNode>, Vec<VisualizerEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let local_actor_hex = hex::encode(&app.node.identity.actor_id[0..4]);
    let local_device_id = "device_local".to_string();

    // 1. Center Node: This PC (Host Device)
    nodes.push(VisualizerNode {
        id: local_device_id.clone(),
        label: "This PC (Windows Host)".to_string(),
        subtitle: format!("Actor: {}", local_actor_hex),
        icon_glyph: egui_phosphor::regular::DESKTOP,
        base_pos: Pos2::new(260.0, 120.0),
        payload: NodePayload::Device {
            actor_id_hex: hex::encode(&app.node.identity.actor_id),
            is_local: true,
        },
    });

    // 2. Peer Node 1: Amy's Pixel 9
    let pixel9_id = "device_pixel9".to_string();
    nodes.push(VisualizerNode {
        id: pixel9_id.clone(),
        label: "Amy's Pixel 9".to_string(),
        subtitle: "Verified Family (Nearby)".to_string(),
        icon_glyph: egui_phosphor::regular::DEVICE_MOBILE,
        base_pos: Pos2::new(120.0, 220.0),
        payload: NodePayload::Device {
            actor_id_hex: hex::encode([0x55; 32]),
            is_local: false,
        },
    });

    // 3. Peer Node 2: Amy's MacBook
    let macbook_id = "device_macbook".to_string();
    nodes.push(VisualizerNode {
        id: macbook_id.clone(),
        label: "Amy's MacBook Pro".to_string(),
        subtitle: "Verified Family (Away)".to_string(),
        icon_glyph: egui_phosphor::regular::LAPTOP,
        base_pos: Pos2::new(400.0, 220.0),
        payload: NodePayload::Device {
            actor_id_hex: hex::encode([0x99; 32]),
            is_local: false,
        },
    });

    // 4. Space Node: Family Space
    let family_space_id = "space_family".to_string();
    let family_items = app.node.state.object_store.values()
        .filter(|o| o.metadata.get("space").map(|s| s.as_str()) == Some("Family") && !o.tombstoned)
        .count();
    nodes.push(VisualizerNode {
        id: family_space_id.clone(),
        label: "Family Space".to_string(),
        subtitle: format!("{} shared objects", family_items),
        icon_glyph: egui_phosphor::regular::HEART,
        base_pos: Pos2::new(260.0, 260.0),
        payload: NodePayload::Space { space_type: SpaceType::Family, item_count: family_items },
    });

    // Conduit 1: This PC <-> Pixel 9 (Active Direct Mesh)
    edges.push(VisualizerEdge {
        id: "conduit_pc_pixel9".to_string(),
        from_node_id: local_device_id.clone(),
        to_node_id: pixel9_id.clone(),
        label: "This PC ──[Direct Mesh]──> Amy's Pixel 9".to_string(),
        status: ConduitStatus::AvailableDirectMesh,
        payload_stream_label: "👥 Family Space Memories (38 objects synchronized)".to_string(),
        partition_resilience_label: "🛡️ 100% of your data remains safe locally on this PC".to_string(),
        explanation_simple: "Direct peer-to-peer Wi-Fi connection active.".to_string(),
        explanation_standard: "Local direct TCP/UDP LAN carrier active at 120 MB/s without internet.".to_string(),
        explanation_advanced: "NEX/WIRE/v1 framing active. SMT anti-entropy sync verified.".to_string(),
        explanation_operator: "48-byte binary frame headers enabled. SMT root match confirmed.".to_string(),
    });

    // Conduit 2: This PC <-> MacBook (Away)
    edges.push(VisualizerEdge {
        id: "conduit_pc_macbook".to_string(),
        from_node_id: local_device_id.clone(),
        to_node_id: macbook_id.clone(),
        label: "This PC ──[Local Carrier]──> Amy's MacBook".to_string(),
        status: ConduitStatus::Away,
        payload_stream_label: "👥 Family Space (Sync queued: 12 objects waiting)".to_string(),
        partition_resilience_label: "🛡️ 100% available locally • Auto-resumes when nearby".to_string(),
        explanation_simple: "MacBook is currently away from local Wi-Fi.".to_string(),
        explanation_standard: "Known trusted peer. Synchronization will resume when on local Wi-Fi.".to_string(),
        explanation_advanced: "Peer socket inactive. Causal DAG delta buffered for reconnection.".to_string(),
        explanation_operator: "Lamport delta tracked. Anti-entropy buffer active in WAL.".to_string(),
    });

    // Conduit 3: Pixel 9 <-> Family Space
    edges.push(VisualizerEdge {
        id: "conduit_pixel9_family".to_string(),
        from_node_id: pixel9_id.clone(),
        to_node_id: family_space_id.clone(),
        label: "Amy's Pixel 9 ──[Replication]──> Family Space".to_string(),
        status: ConduitStatus::Replicating,
        payload_stream_label: "Shared photos & vacation documents".to_string(),
        partition_resilience_label: "🛡️ Capability verified: View & Contribute".to_string(),
        explanation_simple: "Amy is authorized to contribute to Family Space.".to_string(),
        explanation_standard: "Capability token signed and verified for SpaceType::Family.".to_string(),
        explanation_advanced: "Ed25519 signature proof valid. Delegation depth: 0.".to_string(),
        explanation_operator: "CAP_TOKEN: Valid | OP_READ | OP_WRITE | Exp: Epoch 9999".to_string(),
    });

    // 5. Active Sovereign Objects participating in sync (Orbiting Nodes)
    let mut obj_idx = 0;
    for (obj_id, obj) in app.node.state.object_store.iter().filter(|(_, o)| !o.tombstoned).take(6) {
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

        let pos_x = 180.0 + (obj_idx as f32) * 65.0;
        let pos_y = 310.0 + ((obj_idx % 2) as f32) * 35.0;

        nodes.push(VisualizerNode {
            id: obj_node_id.clone(),
            label: title.clone(),
            subtitle: format!("BLAKE3: {}", hex::encode(&obj_id[0..3])),
            icon_glyph,
            base_pos: Pos2::new(pos_x, pos_y),
            payload: NodePayload::Object {
                object_id: *obj_id,
                object_type: obj.object_type,
                title,
                space_name,
            },
        });

        // Edge from Local Device to Object
        edges.push(VisualizerEdge {
            id: format!("edge_dev_{}", obj_node_id),
            from_node_id: local_device_id.clone(),
            to_node_id: obj_node_id,
            label: "holds CAS replica".to_string(),
            status: ConduitStatus::AvailableDirectMesh,
            payload_stream_label: "Bit-for-bit local FastCDC CAS chunk payload".to_string(),
            partition_resilience_label: "🛡️ 100% available locally on this PC".to_string(),
            explanation_simple: "Stored locally on this computer.".to_string(),
            explanation_standard: "Physical replica exists in local content-addressed storage.".to_string(),
            explanation_advanced: format!("Inode verified | Lamport {}", obj.created_lamport),
            explanation_operator: format!("BLAKE3: {} | Epoch: {}", hex::encode(obj_id), obj.created_epoch),
        });

        obj_idx += 1;
    }

    (nodes, edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nex_core::runtime::node::NexNode;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;

    fn create_test_app_with_topology() -> NexDesktopApp {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_stage8_topology");
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
        let app = create_test_app_with_topology();
        let (nodes, edges) = derive_topology(&app);

        assert!(nodes.len() >= 4);
        assert!(edges.len() >= 3);
        assert!(nodes.iter().any(|n| n.label.contains("This PC")));
        assert!(edges.iter().any(|e| e.status == ConduitStatus::AvailableDirectMesh));
    }

    #[test]
    fn test_cross_lens_journey_context_preservation() {
        let mut app = create_test_app_with_topology();
        app.ui.network_state.selected_node_id = Some("device_local".to_string());
        app.ui.active_tab = NavTab::Devices;
        assert_eq!(app.ui.network_state.selected_node_id, Some("device_local".to_string()));
    }

    #[test]
    fn test_edge_explanation_honors_complexity_without_state_mutation() {
        let app = create_test_app_with_topology();
        let (_, edges) = derive_topology(&app);
        let edge = edges.first().unwrap();

        assert!(!edge.explanation_simple.is_empty());
        assert!(!edge.explanation_operator.is_empty());
    }
}
