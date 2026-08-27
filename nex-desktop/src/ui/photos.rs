use egui::{Ui, RichText, Frame, Button, Sense};
use std::path::Path;
use ed25519_dalek::Signer;
use nex_core::runtime::experience::HumanExperienceEngine;
use nex_core::runtime::shell::{SpaceType, NexHomeShell};
use nex_core::product::ingest::LocalFileIngestor;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, OP_WRITE};
use nex_core::identity::verifier::hash_capability_token;
use crate::app::NexDesktopApp;
use crate::ui::{palette, inspector::SelectedEntity};

pub fn render(ui: &mut Ui, app: &mut NexDesktopApp) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(RichText::new("Photos").size(28.0).strong().color(palette::TEXT));
            ui.label(RichText::new("Your private sovereign photo library").color(palette::TEXT_DIM).size(14.0));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(format!("{}  Import Photo", egui_phosphor::regular::PLUS)).clicked() {
                if let Some(path) = pick_file() {
                    let actor_id = app.node.identity.actor_id;
                    let epoch = app.node.state.current_epoch;
                    let ns = NexHomeShell::space_to_namespace(SpaceType::Personal);

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
                        SpaceType::Personal,
                        Path::new(&path),
                        &proof,
                        &actor_id,
                        epoch,
                    ) {
                        Ok(_) => {
                            let name = path.split(['/', '\\']).last().unwrap_or(&path).to_string();
                            app.ui.status_msg = format!("Photo added: {}", name);
                        }
                        Err(e) => {
                            app.ui.status_msg = format!("Error: {}", e);
                        }
                    }
                }
            }
        });
    });

    ui.add_space(14.0);

    let vm = HumanExperienceEngine::render_photos_screen(
        &app.node, SpaceType::Personal, app.ui.complexity
    );

    // Storage and sync status bar
    Frame::new()
        .fill(palette::PANEL)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} Storage: {}", egui_phosphor::regular::DATABASE, vm.storage_used_label)).color(palette::TEXT).size(12.5));
                ui.separator();
                ui.label(RichText::new(format!("{} {}", egui_phosphor::regular::SHIELD_CHECK, vm.sync_status_label)).color(palette::ACCENT_GREEN).size(12.5));
            });
        });
    ui.add_space(12.0);

    if vm.photo_cards.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Image::new(egui::include_image!("../../assets/nex_brand_icon.png")).max_height(48.0).max_width(48.0));
            ui.add_space(12.0);
            ui.label(RichText::new("No photos in sovereign library").size(18.0).strong().color(palette::TEXT));
            ui.add_space(4.0);
            ui.label(RichText::new("Import a photo to establish physical replicas across your mesh")
                .size(13.0).color(palette::TEXT_DIM));
        });
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for card in &vm.photo_cards {
                let obj_id = card.object_id;
                let is_selected = app.ui.selected_entity == Some(SelectedEntity::Object(obj_id));
                let card_bg = if is_selected { palette::SELECTED } else { palette::PANEL };

                let response = Frame::new()
                    .fill(card_bg)
                    .corner_radius(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(egui_phosphor::regular::IMAGE).size(22.0).color(palette::ACCENT));
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&card.title).strong().size(14.5).color(palette::TEXT));
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&card.byte_size_formatted)
                                        .size(12.0).color(palette::TEXT_DIM));
                                    ui.separator();
                                    ui.label(RichText::new(&card.status_badge)
                                        .size(12.0).color(palette::ACCENT_GREEN));
                                });
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(format!("{} Inspect", egui_phosphor::regular::MAGNIFYING_GLASS))
                                    .size(12.0).color(palette::ACCENT));
                            });
                        });
                    });

                if response.response.interact(Sense::click()).clicked() {
                    app.ui.selected_entity = Some(SelectedEntity::Object(obj_id));
                }
                ui.add_space(6.0);
            }
        });
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
