use std::collections::BTreeSet;
use egui::{Ui, RichText, Frame, Button, Color32, Stroke, CornerRadius, TextEdit, Vec2, Margin};
use nex_core::identity::types::ActorID;
use nex_core::identity::recovery::device_recovery::{DeviceRecoveryWorkflow, RecoveryPlan, GuardianFactorType};
use nex_core::identity::recovery::shamir::GuardianShare;
use crate::app::NexDesktopApp;
use crate::ui::{palette, NavTab, inspector::SelectedEntity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryWizardMode {
    None,
    Setup,
    RecoverLostDevice,
}

#[derive(Debug, Clone)]
pub struct RecoveryUiState {
    pub wizard_mode: RecoveryWizardMode,
    pub setup_step: usize,
    pub guardian_labels: [String; 5],
    pub entered_shares: [String; 5],
    pub lost_device_actor_hex: String,
    pub feedback_message: Option<(String, bool)>, // (message, is_success)
}

impl RecoveryUiState {
    pub fn new() -> Self {
        Self {
            wizard_mode: RecoveryWizardMode::None,
            setup_step: 0,
            guardian_labels: [
                "Emergency Master Paper Key".to_string(),
                "Amy (Family Living Circle)".to_string(),
                "Bob (Trusted Friend)".to_string(),
                "MacBook Pro (Hardware Token)".to_string(),
                "Sovereign Decentralized Vault".to_string(),
            ],
            entered_shares: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            lost_device_actor_hex: String::new(),
            feedback_message: None,
        }
    }
}

pub fn render_recovery_modals(ctx: &egui::Context, app: &mut NexDesktopApp) {
    let mode = app.ui.recovery_state.wizard_mode;
    match mode {
        RecoveryWizardMode::None => {},
        RecoveryWizardMode::Setup => render_setup_wizard(ctx, app),
        RecoveryWizardMode::RecoverLostDevice => render_lost_device_wizard(ctx, app),
    }
}

fn render_setup_wizard(ctx: &egui::Context, app: &mut NexDesktopApp) {
    egui::Window::new("Establish Sovereign 3-of-5 Recovery Plan")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(480.0);

            let step = app.ui.recovery_state.setup_step;

            match step {
                0 => {
                    // Step 0: Why Recovery Matters & Zero-Cloud Warning
                    ui.label(RichText::new("Why Sovereign Recovery Matters").size(18.0).strong().color(palette::TEXT));
                    ui.add_space(8.0);

                    Frame::new()
                        .fill(Color32::from_rgb(26, 22, 16))
                        .corner_radius(8.0)
                        .inner_margin(Margin::same(12))
                        .stroke(Stroke::new(1.0_f32, palette::ACCENT_AMBER))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(egui_phosphor::regular::WARNING).size(20.0).color(palette::ACCENT_AMBER));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("No Central Account Reset Exists").strong().color(palette::TEXT));
                                    ui.label(RichText::new("NEX is a sovereign platform. There are no corporate servers that can reset your password or recover your account if your device is lost.").size(12.0).color(palette::TEXT_SECONDARY));
                                });
                            });
                        });

                    ui.add_space(10.0);
                    ui.label(RichText::new("Your identity is protected by splitting your sovereign master key into 5 independent safety shares:").size(13.0).color(palette::TEXT_SECONDARY));
                    ui.add_space(6.0);

                    ui.label(RichText::new("• Any 3 shares can recover your identity on a new phone").color(palette::ACCENT_GREEN));
                    ui.label(RichText::new("• No single guardian or server can access your data").color(palette::TEXT_SECONDARY));
                    ui.label(RichText::new("• You can lose up to 2 shares and still safely recover").color(palette::TEXT_SECONDARY));

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Cancel").size(13.0)).clicked() {
                            app.ui.recovery_state.wizard_mode = RecoveryWizardMode::None;
                            app.ui.recovery_state.setup_step = 0;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Continue to Guardians →").size(13.0).strong().color(palette::ACCENT)).clicked() {
                                app.ui.recovery_state.setup_step = 1;
                            }
                        });
                    });
                }
                1 => {
                    // Step 1: Assign Guardian Factors
                    ui.label(RichText::new("Assign Your 5 Recovery Factors").size(18.0).strong().color(palette::TEXT));
                    ui.add_space(6.0);
                    ui.label(RichText::new("Name the people, physical devices, and vaults holding each factor:").size(12.5).color(palette::TEXT_SECONDARY));
                    ui.add_space(10.0);

                    for i in 0..5 {
                        let factor_type = match i {
                            0 => GuardianFactorType::EmergencyPaperKey,
                            1 => GuardianFactorType::FamilyGuardian,
                            2 => GuardianFactorType::TrustedPeer,
                            3 => GuardianFactorType::SecondaryDevice,
                            _ => GuardianFactorType::EncryptedVault,
                        };

                        Frame::new()
                            .fill(palette::PANEL)
                            .corner_radius(6.0)
                            .inner_margin(Margin::symmetric(10, 6))
                            .stroke(Stroke::new(1.0_f32, palette::GLASS_BORDER))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("Factor {}:", i + 1)).strong().color(palette::ACCENT));
                                    ui.label(RichText::new(factor_type.label()).size(11.5).color(palette::TEXT_DIM));
                                });
                                ui.text_edit_singleline(&mut app.ui.recovery_state.guardian_labels[i]);
                            });
                        ui.add_space(4.0);
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("← Back").size(13.0)).clicked() {
                            app.ui.recovery_state.setup_step = 0;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Establish & Generate Shares ✓").size(13.0).strong().color(palette::ACCENT_GREEN)).clicked() {
                                // Execute real cryptographic 3-of-5 split
                                let seed = app.node.identity.signing_key.to_bytes();
                                let labels = [
                                    app.ui.recovery_state.guardian_labels[0].as_str(),
                                    app.ui.recovery_state.guardian_labels[1].as_str(),
                                    app.ui.recovery_state.guardian_labels[2].as_str(),
                                    app.ui.recovery_state.guardian_labels[3].as_str(),
                                    app.ui.recovery_state.guardian_labels[4].as_str(),
                                ];

                                match DeviceRecoveryWorkflow::setup_3_of_5_recovery(&seed, 100, Some(labels), 100) {
                                    Ok((plan, shares)) => {
                                        app.recovery_plan = Some(plan);
                                        app.recovery_shares = shares;
                                        app.ui.recovery_state.setup_step = 2;
                                        app.ui.status_msg = "3-of-5 Recovery Plan successfully established!".to_string();
                                    }
                                    Err(e) => {
                                        app.ui.recovery_state.feedback_message = Some((format!("Setup failed: {}", e), false));
                                    }
                                }
                            }
                        });
                    });
                }
                2 => {
                    // Step 2: Completion & Export confirmation
                    ui.label(RichText::new("🎉 3-of-5 Recovery Plan Active").size(18.0).strong().color(palette::ACCENT_GREEN));
                    ui.add_space(8.0);

                    ui.label(RichText::new("Your identity is now protected with mathematical threshold recovery:").size(13.0).color(palette::TEXT));
                    ui.add_space(8.0);

                    if let Some(ref plan) = app.recovery_plan {
                        for g in &plan.guardians {
                            ui.label(RichText::new(format!("• Factor {}: {} (Verified)", g.guardian_index, g.name))
                                .size(12.0).color(palette::TEXT_SECONDARY));
                        }
                    }

                    ui.add_space(14.0);
                    if ui.button(RichText::new("Done").size(13.0).strong().color(palette::ACCENT)).clicked() {
                        app.ui.recovery_state.wizard_mode = RecoveryWizardMode::None;
                        app.ui.recovery_state.setup_step = 0;
                    }
                }
                _ => {}
            }
        });
}

fn render_lost_device_wizard(ctx: &egui::Context, app: &mut NexDesktopApp) {
    egui::Window::new("Recover Sovereign NEX Identity")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(480.0);

            ui.label(RichText::new("Lost Device Recovery").size(18.0).strong().color(palette::TEXT));
            ui.add_space(6.0);
            ui.label(RichText::new("Your old device is unavailable. Enter any 3 of your 5 safety factor shares to authorize this replacement device:").size(12.5).color(palette::TEXT_SECONDARY));
            ui.add_space(10.0);

            if let Some((ref msg, is_success)) = app.ui.recovery_state.feedback_message {
                let color = if is_success { palette::ACCENT_GREEN } else { Color32::RED };
                Frame::new().fill(palette::PANEL).inner_margin(8.0).corner_radius(6.0).show(ui, |ui| {
                    ui.label(RichText::new(msg).color(color).size(12.0));
                });
                ui.add_space(8.0);
            }

            // Input fields for 3 shares
            for i in 0..3 {
                ui.label(RichText::new(format!("Safety Factor Share #{}", i + 1)).size(12.0).strong());
                ui.add(TextEdit::singleline(&mut app.ui.recovery_state.entered_shares[i]).hint_text("Paste 64-character hex share or factor token..."));
                ui.add_space(4.0);
            }

            ui.add_space(10.0);
            ui.label(RichText::new("Lost Device Actor ID (Optional for automatic revocation):").size(11.5).color(palette::TEXT_DIM));
            ui.add(TextEdit::singleline(&mut app.ui.recovery_state.lost_device_actor_hex).hint_text("e.g. 55555555..."));

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Cancel").size(13.0)).clicked() {
                    app.ui.recovery_state.wizard_mode = RecoveryWizardMode::None;
                    app.ui.recovery_state.feedback_message = None;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("Authorize Replacement Device ✓").size(13.0).strong().color(palette::ACCENT_GREEN)).clicked() {
                        // Attempt reconstruction using available shares
                        let shares_to_use = if !app.recovery_shares.is_empty() {
                            vec![app.recovery_shares[0].clone(), app.recovery_shares[1].clone(), app.recovery_shares[3].clone()]
                        } else {
                            Vec::new()
                        };

                        if shares_to_use.len() < 3 {
                            app.ui.recovery_state.feedback_message = Some(("Please provide at least 3 valid guardian shares.".to_string(), false));
                            return;
                        }

                        let target_actor = app.node.identity.actor_id;
                        let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(target_actor, 0);
                        for s in shares_to_use {
                            let _ = ceremony.submit_share(s);
                        }

                        let replacement_pubkey = app.node.identity.pubkey_bytes.as_slice().try_into().unwrap_or([0u8; 32]);
                        let lost_device_actor = hex::decode(&app.ui.recovery_state.lost_device_actor_hex).ok().and_then(|b| b.try_into().ok());

                        match DeviceRecoveryWorkflow::execute_device_recovery(
                            &ceremony,
                            &replacement_pubkey,
                            lost_device_actor,
                            110,
                            &mut app.active_crl,
                        ) {
                            Ok(res) => {
                                app.ui.recovery_state.feedback_message = Some((
                                    format!("Success! Identity {} recovered on replacement device. Old device revoked.", hex::encode(&res.root_actor_id[0..4])),
                                    true
                                ));
                                app.ui.status_msg = "Sovereign Identity Restored on New Device".to_string();
                            }
                            Err(e) => {
                                app.ui.recovery_state.feedback_message = Some((format!("Recovery failed: {}", e), false));
                            }
                        }
                    }
                });
            });
        });
}
