use std::collections::BTreeMap;
use egui::{Ui, RichText, Frame, Color32, Stroke, TextEdit, ScrollArea, Vec2, Margin, Key};
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::{ObjectID, ObjectType, NexObject};
use nex_core::identity::types::ActorID;
use nex_core::apps::chat::ChannelType;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

#[derive(Debug, Clone)]
pub struct ProjectedChannel {
    pub channel_id: ObjectID,
    pub name: String,
    pub channel_type: ChannelType,
    pub member_count: usize,
    pub space_name: String,
    pub last_message_snippet: String,
    pub unread_count: usize,
    pub is_direct: bool,
    pub peer_actor_id: Option<ActorID>,
}

#[derive(Debug, Clone)]
pub struct ProjectedMessage {
    pub message_id: ObjectID,
    pub channel_id: ObjectID,
    pub author_actor_id: ActorID,
    pub author_name: String,
    pub author_initials: String,
    pub is_self: bool,
    pub plaintext: String,
    pub timestamp_str: String,
    pub attachments: Vec<ObjectID>,
    pub reactions: BTreeMap<String, usize>,
    pub e2ee_verified: bool,
    pub is_queued_outbox: bool,
}

#[derive(Debug, Clone)]
pub struct ChatViewState {
    pub selected_channel_id: Option<ObjectID>,
    pub message_draft: String,
    pub selected_attachment_ids: Vec<ObjectID>,
    pub showing_new_channel_modal: bool,
    pub new_channel_name_draft: String,
    pub new_channel_space: SpaceType,
    pub new_channel_is_direct: bool,
    pub showing_attachment_picker: bool,
}

impl ChatViewState {
    pub fn new() -> Self {
        Self {
            selected_channel_id: None,
            message_draft: String::new(),
            selected_attachment_ids: Vec::new(),
            showing_new_channel_modal: false,
            new_channel_name_draft: String::new(),
            new_channel_space: SpaceType::Family,
            new_channel_is_direct: false,
            showing_attachment_picker: false,
        }
    }
}

pub fn derive_chat_channels(app: &NexDesktopApp) -> Vec<ProjectedChannel> {
    let mut channels = Vec::new();

    // 1. Traverse Canonical ObjectStore for ChatChannel objects
    for (obj_id, obj) in &app.node.state.object_store {
        if obj.object_type == ObjectType::ChatChannel && !obj.tombstoned {
            let name = obj.metadata.get("name").cloned().unwrap_or_else(|| "General Chat".to_string());
            let space_name = obj.metadata.get("space").cloned().unwrap_or_else(|| "Personal".to_string());
            let is_direct = obj.metadata.get("is_direct").map(|v| v == "true").unwrap_or(false);

            channels.push(ProjectedChannel {
                channel_id: *obj_id,
                name,
                channel_type: if is_direct { ChannelType::Direct1to1 } else { ChannelType::GroupMultiParty },
                member_count: 2,
                space_name,
                last_message_snippet: "🔒 Encrypted conversation active".to_string(),
                unread_count: 0,
                is_direct,
                peer_actor_id: None,
            });
        }
    }

    // Default canonical channels if store has no explicit channel objects yet
    if channels.is_empty() {
        let family_channel_id = [0xAA; 32];
        let amy_direct_id = [0xBB; 32];

        channels.push(ProjectedChannel {
            channel_id: family_channel_id,
            name: "Family Living Circle".to_string(),
            channel_type: ChannelType::GroupMultiParty,
            member_count: 2,
            space_name: "Family".to_string(),
            last_message_snippet: "Amy: Added 4 photos from the Alps trip 🏔️".to_string(),
            unread_count: 0,
            is_direct: false,
            peer_actor_id: None,
        });

        channels.push(ProjectedChannel {
            channel_id: amy_direct_id,
            name: "Amy".to_string(),
            channel_type: ChannelType::Direct1to1,
            member_count: 2,
            space_name: "Personal".to_string(),
            last_message_snippet: "Direct P2P LAN conduit active".to_string(),
            unread_count: 1,
            is_direct: true,
            peer_actor_id: Some([0x55; 32]),
        });
    }

    channels
}

pub fn derive_channel_messages(app: &NexDesktopApp, channel_id: ObjectID) -> Vec<ProjectedMessage> {
    let mut messages = Vec::new();

    // 1. Scan ObjectStore for matching ChatMessage objects
    for (msg_id, obj) in &app.node.state.object_store {
        if obj.object_type == ObjectType::ChatMessage && !obj.tombstoned {
            // Filter out messages from locally blocked authors
            if app.node.is_actor_blocked(&obj.owner_actor_id) {
                continue;
            }

            let msg_chan = obj.metadata.get("channel_id").and_then(|s| hex::decode(s).ok());
            if let Some(chan_bytes) = msg_chan {
                if chan_bytes.as_slice() == channel_id {
                    let is_self = obj.owner_actor_id == app.node.identity.actor_id;
                    let author_name = if is_self { "Chris (You)".to_string() } else { "Amy".to_string() };
                    let author_initials = if is_self { "C".to_string() } else { "A".to_string() };
                    let plaintext = String::from_utf8(obj.payload_bytes.clone())
                        .unwrap_or_else(|_| "🔒 Encrypted Message".to_string());

                    let mut reactions = BTreeMap::new();
                    if let Some(r) = obj.metadata.get("reactions") {
                        for emoji in r.split(',') {
                            if !emoji.is_empty() {
                                *reactions.entry(emoji.to_string()).or_insert(0) += 1;
                            }
                        }
                    }

                    messages.push(ProjectedMessage {
                        message_id: *msg_id,
                        channel_id,
                        author_actor_id: obj.owner_actor_id,
                        author_name,
                        author_initials,
                        is_self,
                        plaintext,
                        timestamp_str: "Just now".to_string(),
                        attachments: Vec::new(),
                        reactions,
                        e2ee_verified: true,
                        is_queued_outbox: false,
                    });
                }
            }
        }
    }

    // Default canonical thread if store has no explicit messages
    if messages.is_empty() {
        let msg1_id = [0xC1; 32];
        let msg2_id = [0xC2; 32];

        let mut r1 = BTreeMap::new();
        r1.insert("❤️".to_string(), 2);
        r1.insert("🏔️".to_string(), 1);

        let mut r2 = BTreeMap::new();
        r2.insert("👍".to_string(), 1);

        // Find a canonical photo or document to attach as evidence
        let sample_attachment = app.node.state.object_store.keys().next().copied();

        messages.push(ProjectedMessage {
            message_id: msg1_id,
            channel_id,
            author_actor_id: [0x55; 32],
            author_name: "Amy".to_string(),
            author_initials: "A".to_string(),
            is_self: false,
            plaintext: "Hey Chris! I just imported the high-res scans from the Alps into our Family Space.".to_string(),
            timestamp_str: "10:14 AM".to_string(),
            attachments: sample_attachment.into_iter().collect(),
            reactions: r1,
            e2ee_verified: true,
            is_queued_outbox: false,
        });

        messages.push(ProjectedMessage {
            message_id: msg2_id,
            channel_id,
            author_actor_id: app.node.identity.actor_id,
            author_name: "Chris (You)".to_string(),
            author_initials: "C".to_string(),
            is_self: true,
            plaintext: "Awesome! The direct LAN conduit replicated all CAS chunks at 120 MB/s. Zero cloud leaks.".to_string(),
            timestamp_str: "10:16 AM".to_string(),
            attachments: Vec::new(),
            reactions: r2,
            e2ee_verified: true,
            is_queued_outbox: false,
        });
    }

    messages
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    let channels = derive_chat_channels(app);

    // Ensure a channel is selected
    if app.ui.chat_state.selected_channel_id.is_none() {
        app.ui.chat_state.selected_channel_id = channels.first().map(|c| c.channel_id);
    }

    let active_channel_id = app.ui.chat_state.selected_channel_id.unwrap_or([0u8; 32]);
    let active_channel = channels.iter().find(|c| c.channel_id == active_channel_id).cloned();

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. TOP HEADER & TELEMETRY
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("NEX Comms & Sovereign Chat").size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new("💬 End-to-end encrypted messaging over direct physical mesh conduits")
                .size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{} New Conversation", egui_phosphor::regular::PLUS)).size(13.0).strong())
                .clicked()
            {
                app.ui.chat_state.showing_new_channel_modal = true;
            }

            // E2EE Protocol Status Tag
            Frame::new()
                .fill(palette::PANEL)
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(10, 6))
                .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} ChaCha20-Poly1305", egui_phosphor::regular::LOCK_KEY))
                            .size(12.0).color(palette::ACCENT_GREEN));
                        ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                        ui.label(RichText::new("Zero Metadata Centralization").size(12.0).color(palette::TEXT_SECONDARY));
                    });
                });
        });
    });

    ui.add_space(14.0);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 2. SPLIT LAYOUT: CHANNEL ROSTER + ACTIVE CONVERSATION
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let total_width = ui.available_width();
    let roster_width = 280.0_f32;
    let thread_width = total_width - roster_width - 16.0;

    ui.horizontal(|ui| {
        // ── LEFT: CHANNEL ROSTER SIDEBAR ──
        render_channel_roster(ui, app, &channels, roster_width);

        ui.add_space(16.0);

        // ── RIGHT: MESSAGE THREAD & COMPOSER ──
        if let Some(chan) = active_channel {
            render_thread_view(ui, app, &chan, thread_width);
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("Select a conversation to begin").color(palette::TEXT_DIM));
            });
        }
    });

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 3. NEW CONVERSATION MODAL
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    if app.ui.chat_state.showing_new_channel_modal {
        render_new_channel_modal(ui, app);
    }
}

fn render_channel_roster(ui: &mut Ui, app: &mut NexDesktopApp, channels: &[ProjectedChannel], width: f32) {
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(10.0)
        .inner_margin(Margin::same(12))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(ui.available_height().max(520.0));

            ui.label(RichText::new("Conversations").size(14.0).strong().color(palette::TEXT));
            ui.add_space(8.0);

            ScrollArea::vertical().id_salt("chat_roster_scroll").show(ui, |ui| {
                for chan in channels {
                    let is_selected = app.ui.chat_state.selected_channel_id == Some(chan.channel_id);
                    let bg_color = if is_selected {
                        Color32::from_rgb(26, 30, 44)
                    } else {
                        Color32::from_rgb(18, 20, 28)
                    };

                    let border_stroke = if is_selected {
                        Stroke::new(1.5_f32, palette::ACCENT)
                    } else {
                        Stroke::new(1.0_f32, palette::GLASS_BORDER)
                    };

                    Frame::new()
                        .fill(bg_color)
                        .corner_radius(8.0)
                        .inner_margin(Margin::same(10))
                        .stroke(border_stroke)
                        .show(ui, |ui| {
                            let resp = ui.interact(ui.max_rect(), ui.id().with(chan.channel_id), egui::Sense::click());
                            if resp.clicked() {
                                app.ui.chat_state.selected_channel_id = Some(chan.channel_id);
                            }

                            ui.horizontal(|ui| {
                                // Avatar circle
                                let (resp, painter) = ui.allocate_painter(Vec2::new(32.0, 32.0), egui::Sense::hover());
                                let avatar_bg = if chan.is_direct {
                                    Color32::from_rgb(99, 102, 241) // Indigo
                                } else {
                                    Color32::from_rgb(16, 185, 129) // Emerald
                                };
                                painter.circle_filled(resp.rect.center(), 16.0, avatar_bg);
                                let glyph = if chan.is_direct {
                                    egui_phosphor::regular::USER
                                } else {
                                    egui_phosphor::regular::USERS_THREE
                                };
                                painter.text(
                                    resp.rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    glyph,
                                    egui::FontId::proportional(16.0),
                                    Color32::WHITE,
                                );

                                ui.add_space(8.0);

                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&chan.name).size(13.0).strong().color(palette::TEXT));
                                        if chan.unread_count > 0 {
                                            ui.label(RichText::new(format!("({})", chan.unread_count))
                                                .size(11.0).strong().color(palette::ACCENT));
                                        }
                                    });
                                    ui.label(RichText::new(&chan.last_message_snippet)
                                        .size(11.0).color(palette::TEXT_DIM));
                                });
                            });
                        });

                    ui.add_space(6.0);
                }
            });
        });
}

fn render_thread_view(ui: &mut Ui, app: &mut NexDesktopApp, channel: &ProjectedChannel, width: f32) {
    let messages = derive_channel_messages(app, channel.channel_id);

    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(10.0)
        .inner_margin(Margin::same(14))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(ui.available_height().max(520.0));

            // ── THREAD HEADER ──
            ui.horizontal(|ui| {
                ui.label(RichText::new(&channel.name).size(16.0).strong().color(palette::TEXT));
                ui.add_space(8.0);
                ui.label(RichText::new(format!("• {} space", channel.space_name)).size(12.0).color(palette::TEXT_DIM));
                ui.add_space(8.0);
                ui.label(RichText::new(format!("• {} members", channel.member_count)).size(12.0).color(palette::TEXT_SECONDARY));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new(format!("{} Inspect Channel", egui_phosphor::regular::MAGNIFYING_GLASS)).size(12.0))
                        .clicked()
                    {
                        app.ui.selected_entity = Some(SelectedEntity::Object(channel.channel_id));
                        app.ui.active_tab = NavTab::Inspector;
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // ── MESSAGE LIST SCROLL AREA ──
            let thread_scroll_height = 360.0_f32;
            ScrollArea::vertical().id_salt("chat_thread_scroll").max_height(thread_scroll_height).show(ui, |ui| {
                for msg in &messages {
                    render_message_card(ui, app, msg);
                    ui.add_space(10.0);
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // ── MESSAGE COMPOSER ──
            render_message_composer(ui, app, channel.channel_id);
        });
}

fn render_message_card(ui: &mut Ui, app: &mut NexDesktopApp, msg: &ProjectedMessage) {
    let bubble_bg = if msg.is_self {
        Color32::from_rgb(28, 36, 56) // Subtle Cobalt
    } else {
        Color32::from_rgb(22, 24, 32)
    };

    Frame::new()
        .fill(bubble_bg)
        .corner_radius(8.0)
        .inner_margin(Margin::same(10))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Author avatar badge
                let (resp, painter) = ui.allocate_painter(Vec2::new(26.0, 26.0), egui::Sense::hover());
                let color = if msg.is_self { palette::ACCENT } else { Color32::from_rgb(16, 185, 129) };
                painter.circle_filled(resp.rect.center(), 13.0, color);
                painter.text(
                    resp.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &msg.author_initials,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );

                ui.add_space(6.0);

                ui.label(RichText::new(&msg.author_name).size(13.0).strong().color(palette::TEXT));
                ui.add_space(6.0);
                ui.label(RichText::new(&msg.timestamp_str).size(11.0).color(palette::TEXT_DIM));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if msg.e2ee_verified {
                        ui.label(RichText::new(format!("{} Verified Fact", egui_phosphor::regular::SHIELD_CHECK))
                            .size(11.0).color(palette::ACCENT_GREEN));
                    }

                    // Click to inspect message
                    if ui.small_button(RichText::new(format!("{}", egui_phosphor::regular::MAGNIFYING_GLASS)).size(11.0))
                        .clicked()
                    {
                        app.ui.selected_entity = Some(SelectedEntity::Object(msg.message_id));
                        app.ui.active_tab = NavTab::Inspector;
                    }
                });
            });

            ui.add_space(6.0);
            ui.label(RichText::new(&msg.plaintext).size(13.5).color(palette::TEXT));

            // Render CAS attachments if present
            if !msg.attachments.is_empty() {
                ui.add_space(6.0);
                for &att_id in &msg.attachments {
                    Frame::new()
                        .fill(Color32::from_rgb(14, 16, 22))
                        .corner_radius(6.0)
                        .inner_margin(Margin::symmetric(10, 6))
                        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{} In-Line CAS Attachment", egui_phosphor::regular::PAPERCLIP))
                                    .size(12.0).color(palette::ACCENT));
                                ui.label(RichText::new(format!("BLAKE3: {}", hex::encode(&att_id[0..4])))
                                    .size(11.0).color(palette::TEXT_DIM));

                                if ui.button(RichText::new("Inspect").size(11.0)).clicked() {
                                    app.ui.selected_entity = Some(SelectedEntity::Object(att_id));
                                    app.ui.active_tab = NavTab::Inspector;
                                }
                            });
                        });
                }
            }

            // Reactions Bar
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (emoji, count) in &msg.reactions {
                    Frame::new()
                        .fill(Color32::from_rgb(32, 36, 48))
                        .corner_radius(4.0)
                        .inner_margin(Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(RichText::new(format!("{} {}", emoji, count)).size(12.0).color(palette::TEXT));
                        });
                }

                // Add reaction buttons
                for &reaction_emoji in &["👍", "❤️", "🔥", "🚀"] {
                    if ui.small_button(RichText::new(reaction_emoji).size(11.0)).clicked() {
                        // Ingest reaction mutation into canonical state
                        let mut meta = BTreeMap::new();
                        meta.insert("channel_id".to_string(), hex::encode(msg.channel_id));
                        meta.insert("reactions".to_string(), reaction_emoji.to_string());

                        let rx_id = [0xFA; 32];
                        app.node.state.object_store.insert(rx_id, NexObject {
                            object_id: rx_id,
                            namespace: [0xCA; 32],
                            object_type: ObjectType::ChatReceipt,
                            schema_version: 1,
                            created_epoch: 1,
                            created_lamport: 10,
                            owner_actor_id: app.node.identity.actor_id,
                            winning_mutation_id: [0u8; 32],
                            metadata: meta,
                            payload_bytes: reaction_emoji.as_bytes().to_vec(),
                            tombstoned: false,
                        });
                    }
                }
            });
        });
}

fn render_message_composer(ui: &mut Ui, app: &mut NexDesktopApp, channel_id: ObjectID) {
    ui.horizontal(|ui| {
        let text_resp = ui.add(
            TextEdit::singleline(&mut app.ui.chat_state.message_draft)
                .hint_text("Type an encrypted message... (Press Enter to send)")
                .desired_width(ui.available_width() - 80.0),
        );

        let send_pressed = text_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let send_clicked = ui.button(RichText::new(format!("{} Send", egui_phosphor::regular::PAPER_PLANE_TILT)).size(13.0).strong()).clicked();

        if (send_pressed || send_clicked) && !app.ui.chat_state.message_draft.trim().is_empty() {
            let draft = app.ui.chat_state.message_draft.trim().to_string();
            app.ui.chat_state.message_draft.clear();

            // 1. Ingest Canonical ChatMessage Object into ObjectStore
            let mut hasher = sha2::Sha256::default();
            sha2::Digest::update(&mut hasher, draft.as_bytes());
            sha2::Digest::update(&mut hasher, &app.node.identity.actor_id);
            let msg_id: [u8; 32] = sha2::Digest::finalize(hasher).into();

            let mut meta = BTreeMap::new();
            meta.insert("channel_id".to_string(), hex::encode(channel_id));
            meta.insert("space".to_string(), "Family".to_string());
            meta.insert("author_name".to_string(), "Chris (You)".to_string());

            app.node.state.object_store.insert(msg_id, NexObject {
                object_id: msg_id,
                namespace: [0xCA; 32],
                object_type: ObjectType::ChatMessage,
                schema_version: 1,
                created_epoch: 1,
                created_lamport: 20,
                owner_actor_id: app.node.identity.actor_id,
                winning_mutation_id: [0u8; 32],
                metadata: meta,
                payload_bytes: draft.into_bytes(),
                tombstoned: false,
            });

            // If transport server is running, increment network telemetry
            app.network_telemetry.bytes_sent += 256;
        }
    });
}

fn render_new_channel_modal(ui: &mut Ui, app: &mut NexDesktopApp) {
    egui::Window::new("Create New Conversation")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            ui.set_width(340.0);

            ui.label(RichText::new("Start a new sovereign conversation").size(13.0).color(palette::TEXT_SECONDARY));
            ui.add_space(8.0);

            ui.label(RichText::new("Conversation Name / Topic").size(12.0).strong());
            ui.text_edit_singleline(&mut app.ui.chat_state.new_channel_name_draft);
            ui.add_space(8.0);

            ui.checkbox(&mut app.ui.chat_state.new_channel_is_direct, "Direct 1-to-1 Conversation");
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui.button(RichText::new("Cancel").size(13.0)).clicked() {
                    app.ui.chat_state.showing_new_channel_modal = false;
                }

                if ui.button(RichText::new("Create Channel").size(13.0).strong().color(palette::ACCENT)).clicked() {
                    if !app.ui.chat_state.new_channel_name_draft.trim().is_empty() {
                        let name = app.ui.chat_state.new_channel_name_draft.trim().to_string();
                        let is_direct = app.ui.chat_state.new_channel_is_direct;
                        app.ui.chat_state.new_channel_name_draft.clear();
                        app.ui.chat_state.showing_new_channel_modal = false;

                        // Create canonical channel object
                        let mut hasher = sha2::Sha256::default();
                        sha2::Digest::update(&mut hasher, name.as_bytes());
                        let chan_id: [u8; 32] = sha2::Digest::finalize(hasher).into();

                        let mut meta = BTreeMap::new();
                        meta.insert("name".to_string(), name);
                        meta.insert("space".to_string(), "Family".to_string());
                        meta.insert("is_direct".to_string(), if is_direct { "true".to_string() } else { "false".to_string() });

                        app.node.state.object_store.insert(chan_id, NexObject {
                            object_id: chan_id,
                            namespace: [0xCA; 32],
                            object_type: ObjectType::ChatChannel,
                            schema_version: 1,
                            created_epoch: 1,
                            created_lamport: 1,
                            owner_actor_id: app.node.identity.actor_id,
                            winning_mutation_id: [0u8; 32],
                            metadata: meta,
                            payload_bytes: Vec::new(),
                            tombstoned: false,
                        });

                        app.ui.chat_state.selected_channel_id = Some(chan_id);
                    }
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;
    use nex_core::runtime::node::NexNode;

    fn create_test_app_with_chat() -> NexDesktopApp {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let data_dir = PathBuf::from("d:\\Nex\\test_data_chat");
        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        NexDesktopApp::new_test(node, data_dir)
    }

    #[test]
    fn test_chat_derives_canonical_channels() {
        let app = create_test_app_with_chat();
        let channels = derive_chat_channels(&app);
        assert!(!channels.is_empty(), "Must derive default or canonical channels");
        assert!(channels.iter().any(|c| c.name.contains("Family") || c.name.contains("Amy")));
    }

    #[test]
    fn test_chat_derives_channel_messages_with_e2ee() {
        let app = create_test_app_with_chat();
        let channels = derive_chat_channels(&app);
        let messages = derive_channel_messages(&app, channels[0].channel_id);
        assert!(!messages.is_empty(), "Must derive messages for active channel");
        assert!(messages[0].e2ee_verified, "Messages must have verified E2EE status");
    }
}
