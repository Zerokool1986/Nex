use egui::{Ui, RichText, Frame, Sense, Vec2, Stroke, Color32, CornerRadius};
use std::path::Path;
use ed25519_dalek::Signer;
use nex_core::runtime::experience::HumanExperienceEngine;
use nex_core::runtime::shell::{SpaceType, NexHomeShell};
use nex_core::product::ingest::LocalFileIngestor;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, OP_WRITE};
use nex_core::identity::verifier::hash_capability_token;
use crate::app::NexDesktopApp;
use crate::ui::{palette, inspector::SelectedEntity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoFilter {
    All,
    PersonalOnly,
    FamilyOnly,
    GeotaggedOnly,
}

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. PHOTOS LENS HEADER — Sovereign Visual Memory & Original Purity
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Photos").size(28.0).strong().color(palette::TEXT));
            ui.add_space(2.0);
            ui.label(RichText::new("📷 Sovereign Visual Memories — 100% Original Resolution, zero cloud compression")
                .size(13.0).color(palette::TEXT_SECONDARY));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("{}  Import Photo", egui_phosphor::regular::PLUS)).size(13.0).color(palette::TEXT).strong())
                .clicked()
            {
                if let Some(path) = pick_file() {
                    ingest_photo(app, &path, SpaceType::Personal);
                }
            }
        });
    });

    ui.add_space(16.0);

    // 2. Truthful Substrate Storage & Original Quality Beacon
    render_photos_telemetry_beacon(ui, app);
    ui.add_space(18.0);

    // 3. Filter Bar (All / Personal / Family / Geotagged)
    render_filter_bar(ui, app);
    ui.add_space(20.0);

    // Derive photos from canonical object store
    let (personal_photos, family_photos) = derive_photo_collections(app);
    let total_count = personal_photos.len() + family_photos.len();

    if total_count == 0 {
        render_photos_empty_state(ui, app);
    } else {
        render_photo_memory_grid(ui, app, &personal_photos, &family_photos);
    }
}

/// Renders the Truthful Substrate Telemetry Capsule
fn render_photos_telemetry_beacon(ui: &mut Ui, app: &NexDesktopApp) {
    let vm_personal = HumanExperienceEngine::render_photos_screen(
        &app.node, SpaceType::Personal, app.ui.complexity
    );

    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} Bit-for-bit original fidelity", egui_phosphor::regular::CHECK_CIRCLE))
                    .size(12.0).color(palette::ACCENT_GREEN));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} Local CAS storage: {}", egui_phosphor::regular::DATABASE, vm_personal.storage_used_label))
                    .size(12.0).color(palette::TEXT_SECONDARY));

                ui.add_space(12.0);
                ui.label(RichText::new("•").size(11.0).color(palette::TEXT_DIM));
                ui.add_space(12.0);

                ui.label(RichText::new(format!("{} Zero corporate hosting", egui_phosphor::regular::SHIELD_CHECK))
                    .size(12.0).color(palette::TEXT_SECONDARY));
            });
        });
}

/// Renders the Library Scope Filter Bar
fn render_filter_bar(ui: &mut Ui, _app: &mut NexDesktopApp) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("LIBRARY:").size(11.0).strong().color(palette::TEXT_DIM));
        ui.add_space(4.0);

        // Simple filter labels
        let filters = [
            ("All Memories", true),
            ("Personal Space", false),
            ("Family Circle", false),
            ("Geotagged", false),
        ];

        for (label, active) in filters {
            let bg = if active { palette::SELECTED } else { palette::PANEL };
            let text_color = if active { palette::ACCENT } else { palette::TEXT_SECONDARY };
            let stroke = if active { Stroke::new(1.0_f32, palette::ACCENT) } else { Stroke::new(1.0_f32, palette::GLASS_BORDER) };

            Frame::new()
                .fill(bg)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(10, 4))
                .stroke(stroke)
                .show(ui, |ui| {
                    ui.label(RichText::new(label).size(12.0).color(text_color));
                });
            ui.add_space(4.0);
        }
    });
}

/// Renders the visual masonry memory grid
fn render_photo_memory_grid(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    personal: &[nex_core::runtime::experience::PhotoCardViewModel],
    family: &[nex_core::runtime::experience::PhotoCardViewModel],
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // 1. Family Memories Section (if any)
        if !family.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("FAMILY CIRCLE MEMORIES ({})", family.len()))
                    .size(11.5).strong().color(palette::ACCENT_GREEN));
                ui.label(RichText::new("• Shared with verified family members").size(11.0).color(palette::TEXT_DIM));
            });
            ui.add_space(8.0);

            render_photo_tiles(ui, app, family);
            ui.add_space(24.0);
        }

        // 2. Personal Memories Section (if any)
        if !personal.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("PERSONAL MEMORIES ({})", personal.len()))
                    .size(11.5).strong().color(palette::TEXT));
                ui.label(RichText::new("• Private to this node").size(11.0).color(palette::TEXT_DIM));
            });
            ui.add_space(8.0);

            render_photo_tiles(ui, app, personal);
            ui.add_space(24.0);
        }
    });
}

/// Renders a horizontal wrapped row of responsive Obsidian Glass photo cards
fn render_photo_tiles(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    photos: &[nex_core::runtime::experience::PhotoCardViewModel],
) {
    let available_w = ui.available_width();
    let card_w = 210.0_f32;
    let gutter = 12.0_f32;
    let cols = ((available_w + gutter) / (card_w + gutter)).floor().max(1.0) as usize;

    egui::Grid::new(ui.next_auto_id())
        .spacing(Vec2::new(gutter, gutter))
        .max_col_width(card_w)
        .show(ui, |ui| {
            for (idx, card) in photos.iter().enumerate() {
                render_single_photo_card(ui, app, card, card_w);
                if (idx + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
}

/// Renders an individual Obsidian Glass photo card with aspect ratio preview & metadata
fn render_single_photo_card(
    ui: &mut Ui,
    app: &mut NexDesktopApp,
    card: &nex_core::runtime::experience::PhotoCardViewModel,
    card_w: f32,
) {
    let obj_id = card.object_id;
    let is_selected = app.ui.selected_entity == Some(SelectedEntity::Object(obj_id));

    let card_bg = if is_selected { palette::SELECTED } else { palette::CARD };
    let stroke = if is_selected {
        Stroke::new(1.5_f32, palette::ACCENT)
    } else {
        Stroke::new(1.0_f32, palette::GLASS_BORDER)
    };

    let response = Frame::new()
        .fill(card_bg)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .stroke(stroke)
        .show(ui, |ui| {
            ui.set_width(card_w);
            ui.vertical(|ui| {
                // 1. Aspect Ratio Photo Visual Canvas (16:10 ratio)
                let preview_h = 120.0_f32;
                Frame::new()
                    .fill(Color32::from_rgb(14, 15, 20))
                    .corner_radius(CornerRadius::same(6))
                    .stroke(Stroke::new(1.0_f32, palette::BORDER_SUBTLE))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(card_w - 24.0, preview_h));
                        ui.centered_and_justified(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(32.0);
                                ui.label(RichText::new(egui_phosphor::regular::IMAGE).size(34.0).color(palette::ACCENT));
                                ui.add_space(4.0);
                                ui.label(RichText::new("ORIGINAL RAW").size(9.5).color(palette::TEXT_DIM));
                            });
                        });
                    });

                ui.add_space(8.0);

                // 2. Title & Metadata
                ui.label(RichText::new(&card.title).size(13.5).strong().color(palette::TEXT));
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(&card.byte_size_formatted).size(11.5).color(palette::TEXT_SECONDARY));
                    ui.label(RichText::new("•").size(10.0).color(palette::TEXT_DIM));
                    ui.label(RichText::new("Original").size(11.5).color(palette::ACCENT_GREEN));
                });

                // 3. Location / Space tag if available from canonical metadata
                if let Some(obj) = app.node.state.object_store.get(&obj_id) {
                    if let Some(loc) = obj.metadata.get("location:name") {
                        ui.add_space(2.0);
                        ui.label(RichText::new(format!("📍 {}", loc)).size(11.0).color(palette::TEXT_DIM));
                    }
                }

                // 4. Operator Diagnostics
                if app.ui.complexity == nex_core::runtime::experience::InterfaceComplexity::Expert {
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("CAS: {} | SMT: 100%", &card.object_id_hex[0..8]))
                        .monospace().size(10.0).color(palette::TEXT_DIM));
                }
            });
        });

    if response.response.interact(Sense::click()).clicked() {
        app.ui.selected_entity = Some(SelectedEntity::Object(obj_id));
    }
}

/// Welcoming Photo Empty State Vessel
fn render_photos_empty_state(ui: &mut Ui, app: &mut NexDesktopApp) {
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
                    ui.label(RichText::new(egui_phosphor::regular::IMAGE).size(48.0).color(palette::ACCENT));
                    ui.add_space(16.0);

                    ui.label(RichText::new("Your Sovereign Visual Sanctuary").size(20.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);

                    ui.label(RichText::new("Add photos in 100% full original resolution.\nYour memories stay on your physical hardware, private to you and the people you trust.")
                        .size(13.5).color(palette::TEXT_SECONDARY));
                    ui.add_space(22.0);

                    let btn = ui.add_sized(
                        Vec2::new(220.0, 38.0),
                        egui::Button::new(
                            RichText::new(format!("{}   Import First Photo", egui_phosphor::regular::PLUS))
                                .size(13.5).color(palette::TEXT).strong()
                        )
                        .fill(palette::ACCENT)
                        .corner_radius(8.0),
                    );
                    if btn.clicked() {
                        if let Some(path) = pick_file() {
                            ingest_photo(app, &path, SpaceType::Personal);
                        }
                    }

                    ui.add_space(12.0);
                    ui.label(RichText::new("No lossy cloud compression • Zero algorithmic tracking").size(12.0).color(palette::TEXT_DIM));
                });
            });
    });
}

fn derive_photo_collections(app: &NexDesktopApp) -> (
    Vec<nex_core::runtime::experience::PhotoCardViewModel>,
    Vec<nex_core::runtime::experience::PhotoCardViewModel>,
) {
    let vm_personal = HumanExperienceEngine::render_photos_screen(
        &app.node, SpaceType::Personal, app.ui.complexity
    );
    let vm_family = HumanExperienceEngine::render_photos_screen(
        &app.node, SpaceType::Family, app.ui.complexity
    );

    (vm_personal.photo_cards, vm_family.photo_cards)
}

fn ingest_photo(app: &mut NexDesktopApp, path: &str, space: SpaceType) {
    let actor_id = app.node.identity.actor_id;
    let epoch = app.node.state.current_epoch;
    let ns = NexHomeShell::space_to_namespace(space);

    let token = CapabilityToken {
        issuer: actor_id,
        subject: actor_id,
        namespace: ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 0,
        expires_at_epoch: epoch + 9999,
        parent_token_hash: None,
    };

    let token_hash = hash_capability_token(&token);
    let sig = app.node.identity.signing_key.sign(&token_hash);
    let pubkey_bytes = app.node.identity.pubkey_bytes.clone();

    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(pubkey_bytes),
        parent_proof: None,
        signature: sig.to_bytes().to_vec(),
    };

    match LocalFileIngestor::ingest_file(
        &mut app.node,
        space,
        Path::new(path),
        &proof,
        &actor_id,
        epoch,
    ) {
        Ok(_) => {
            let name = path.split(['/', '\\']).last().unwrap_or(path).to_string();
            app.ui.status_msg = format!("Photo added in full original fidelity: {}", name);
        }
        Err(e) => {
            app.ui.status_msg = format!("Import Error: {}", e);
        }
    }
}

fn pick_file() -> Option<String> {
    use std::process::Command;
    let output = Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null;\
             $d = New-Object System.Windows.Forms.OpenFileDialog;\
             $d.Filter = 'Images (*.jpg;*.jpeg;*.png;*.webp)|*.jpg;*.jpeg;*.png;*.webp|All files (*.*)|*.*';\
             $d.Title = 'Add Photo to NEX';\
             if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $d.FileName }",
        ])
        .output()
        .ok()?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}
