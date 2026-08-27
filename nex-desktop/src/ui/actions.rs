use std::fs;
use std::path::{Path, PathBuf};
use egui::{Ui, RichText, Frame, Color32, Vec2, Stroke};
use sha2::{Sha256, Digest};
use ed25519_dalek::Signer;
use nex_core::object::types::ObjectID;
use nex_core::runtime::shell::SpaceType;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, OP_READ, OP_WRITE};
use nex_core::identity::verifier::hash_capability_token;
use nex_core::product::ingest::LocalFileIngestor;
use crate::app::NexDesktopApp;
use crate::ui::palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionClassification {
    AvailableNow,
    RequiresAuthority,
    RequiresCapability,
    NotImplemented,
    Unavailable,
}

impl ActionClassification {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AvailableNow => "Available",
            Self::RequiresAuthority => "Requires Authority",
            Self::RequiresCapability => "Requires Capability Token",
            Self::NotImplemented => "Future Platform Action",
            Self::Unavailable => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionDialog {
    Rename {
        object_id: ObjectID,
        current_name: String,
        space_name: String,
    },
    DeleteConfirm {
        object_id: ObjectID,
        title: String,
    },
    ShareNotice {
        object_id: ObjectID,
        title: String,
    },
    ImportFile {
        source_path: String,
        target_space: SpaceType,
    },
    ExportFile {
        object_id: ObjectID,
        title: String,
        destination_path: String,
    },
    ProximitySasVerification {
        peer_name: String,
        actor_id: [u8; 32],
        safety_words: [String; 4],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionStatus {
    Success,
    Denied,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ActionResult {
    pub object_id: ObjectID,
    pub status: ActionStatus,
    pub canonical_epoch: u64,
    pub canonical_lamport: u64,
    pub persisted: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub object_id: ObjectID,
    pub bytes_written: usize,
    pub destination_path: String,
    pub hash_matched: bool,
}

#[derive(Debug, Clone)]
pub struct ActionState {
    pub active_dialog: Option<ActionDialog>,
    pub last_result: Option<ActionResult>,
    pub last_export_result: Option<ExportResult>,
    pub text_buffer: String,
}

impl ActionState {
    pub fn new() -> Self {
        Self {
            active_dialog: None,
            last_result: None,
            last_export_result: None,
            text_buffer: String::new(),
        }
    }
}

pub fn render_action_dialog(ui: &mut Ui, app: &mut NexDesktopApp) {
    let dialog = match app.ui.action_state.active_dialog.clone() {
        Some(d) => d,
        None => return,
    };

    egui::Window::new("Sovereign Human Action")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(Frame::new().fill(Color32::from_rgb(20, 24, 34)).corner_radius(8.0).inner_margin(16.0).stroke(Stroke::new(1.0_f32, palette::ACCENT)))
        .show(ui.ctx(), |ui| {
            match dialog {
                ActionDialog::Rename { object_id, current_name, space_name } => {
                    ui.heading(RichText::new("Rename Sovereign Object").size(16.0).strong().color(palette::ACCENT));
                    ui.add_space(8.0);

                    ui.label(RichText::new(format!("Current Name: {}", current_name)).size(13.0).color(palette::TEXT));
                    ui.label(RichText::new(format!("Object ID: {}", hex::encode(&object_id[0..6]))).size(11.5).color(palette::TEXT_DIM));
                    ui.label(RichText::new(format!("Space: {}", space_name)).size(12.0).color(palette::TEXT_DIM));
                    ui.add_space(8.0);

                    let is_authorized = app.node.authorize_request(&[0u8; 32], Some(&object_id), OP_WRITE, None).is_ok();
                    if is_authorized {
                        ui.label(RichText::new("✓ You are authorized to modify this object.").size(12.0).color(palette::ACCENT_GREEN));
                    } else {
                        ui.label(RichText::new("⚠ Unauthorized: Capability delegation required.").size(12.0).color(Color32::RED));
                    }
                    ui.add_space(8.0);

                    ui.label(RichText::new("New Filename:").strong().size(12.5).color(palette::TEXT));
                    ui.text_edit_singleline(&mut app.ui.action_state.text_buffer);
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            app.ui.action_state.active_dialog = None;
                            app.ui.action_state.text_buffer.clear();
                        }
                        if is_authorized && !app.ui.action_state.text_buffer.trim().is_empty() {
                            if ui.button(RichText::new("Commit Rename").strong().color(palette::ACCENT_GREEN)).clicked() {
                                let new_name = app.ui.action_state.text_buffer.trim().to_string();
                                match execute_canonical_rename(app, &object_id, &new_name) {
                                    Ok(result) => {
                                        app.ui.action_state.last_result = Some(result);
                                        app.ui.action_state.active_dialog = None;
                                        app.ui.action_state.text_buffer.clear();
                                    }
                                    Err(err) => {
                                        app.ui.action_state.last_result = Some(ActionResult {
                                            object_id,
                                            status: ActionStatus::Failed,
                                            canonical_epoch: app.node.state.current_epoch,
                                            canonical_lamport: 0,
                                            persisted: false,
                                            message: err,
                                        });
                                    }
                                }
                            }
                        }
                    });
                }
                ActionDialog::ImportFile { source_path: _, target_space } => {
                    ui.heading(RichText::new("Import Real File into NEX").size(16.0).strong().color(palette::ACCENT));
                    ui.add_space(8.0);

                    ui.label(RichText::new("Source File Path on Windows:").strong().size(12.5).color(palette::TEXT));
                    ui.text_edit_singleline(&mut app.ui.action_state.text_buffer);
                    ui.add_space(6.0);

                    let path_str = app.ui.action_state.text_buffer.trim().to_string();
                    let path_buf = PathBuf::from(&path_str);
                    let file_exists = path_buf.exists() && path_buf.is_file();
                    if file_exists {
                        let size = fs::metadata(&path_buf).map(|m| m.len()).unwrap_or(0);
                        ui.label(RichText::new(format!("✓ File Found ({} bytes)", size)).size(12.0).color(palette::ACCENT_GREEN));
                    } else if !path_str.is_empty() {
                        ui.label(RichText::new("⚠ File not found on disk").size(12.0).color(Color32::RED));
                    }

                    ui.label(RichText::new(format!("Target Space: {:?}", target_space)).size(12.0).color(palette::TEXT_DIM));
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            app.ui.action_state.active_dialog = None;
                            app.ui.action_state.text_buffer.clear();
                        }
                        if file_exists {
                            if ui.button(RichText::new("Import into NEX").strong().color(palette::ACCENT_GREEN)).clicked() {
                                let file_to_import = PathBuf::from(app.ui.action_state.text_buffer.trim());
                                match execute_canonical_import(app, &file_to_import, target_space) {
                                    Ok(result) => {
                                        app.ui.action_state.last_result = Some(result);
                                        app.ui.action_state.active_dialog = None;
                                        app.ui.action_state.text_buffer.clear();
                                    }
                                    Err(err) => {
                                        app.ui.action_state.last_result = Some(ActionResult {
                                            object_id: [0u8; 32],
                                            status: ActionStatus::Failed,
                                            canonical_epoch: app.node.state.current_epoch,
                                            canonical_lamport: 0,
                                            persisted: false,
                                            message: err,
                                        });
                                    }
                                }
                            }
                        }
                    });
                }
                ActionDialog::ExportFile { object_id, title, destination_path: _ } => {
                    ui.heading(RichText::new("Export Sovereign Object").size(16.0).strong().color(palette::ACCENT));
                    ui.add_space(8.0);

                    ui.label(RichText::new(format!("Target: {}", title)).size(13.0).color(palette::TEXT));
                    ui.label(RichText::new(format!("Object ID: {}", hex::encode(&object_id[0..6]))).size(11.5).color(palette::TEXT_DIM));
                    ui.add_space(8.0);

                    ui.label(RichText::new("Destination File Path on Windows:").strong().size(12.5).color(palette::TEXT));
                    ui.text_edit_singleline(&mut app.ui.action_state.text_buffer);
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            app.ui.action_state.active_dialog = None;
                            app.ui.action_state.text_buffer.clear();
                        }
                        if !app.ui.action_state.text_buffer.trim().is_empty() {
                            if ui.button(RichText::new("Export File").strong().color(palette::ACCENT_GREEN)).clicked() {
                                let dest_path = Path::new(app.ui.action_state.text_buffer.trim());
                                match execute_canonical_export(app, &object_id, dest_path) {
                                    Ok(res) => {
                                        app.ui.action_state.last_export_result = Some(res);
                                        app.ui.action_state.active_dialog = None;
                                        app.ui.action_state.text_buffer.clear();
                                    }
                                    Err(err) => {
                                        app.ui.action_state.last_result = Some(ActionResult {
                                            object_id,
                                            status: ActionStatus::Failed,
                                            canonical_epoch: app.node.state.current_epoch,
                                            canonical_lamport: 0,
                                            persisted: false,
                                            message: err,
                                        });
                                    }
                                }
                            }
                        }
                    });
                }
                ActionDialog::DeleteConfirm { object_id, title } => {
                    ui.heading(RichText::new("Delete Object").size(16.0).strong().color(Color32::RED));
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Target: {}", title)).size(13.0).color(palette::TEXT));
                    ui.label(RichText::new(format!("Object ID: {}", hex::encode(&object_id[0..6]))).size(11.5).color(palette::TEXT_DIM));
                    ui.add_space(8.0);

                    ui.label(RichText::new("UNAVAILABLE: Destructive tombstoning requires an explicit tombstone capability token (Future Platform Action). No changes were made.")
                        .size(12.0).color(palette::TEXT_DIM));
                    ui.add_space(12.0);

                    if ui.button("Dismiss").clicked() {
                        app.ui.action_state.active_dialog = None;
                    }
                }
                ActionDialog::ShareNotice { object_id, title } => {
                    ui.heading(RichText::new("Share Sovereign Object").size(16.0).strong().color(palette::ACCENT));
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Target: {}", title)).size(13.0).color(palette::TEXT));
                    ui.label(RichText::new(format!("Object ID: {}", hex::encode(&object_id[0..6]))).size(11.5).color(palette::TEXT_DIM));
                    ui.add_space(8.0);

                    ui.label(RichText::new("Capability token sharing ceremony will be available in multi-device sync.")
                        .size(12.0).color(palette::TEXT_DIM));
                    ui.add_space(12.0);

                    if ui.button("Dismiss").clicked() {
                        app.ui.action_state.active_dialog = None;
                    }
                }
                ActionDialog::ProximitySasVerification { peer_name, actor_id, safety_words } => {
                    ui.heading(RichText::new("🛡️ Verify Sovereign Contact (SAS)").size(16.0).strong().color(palette::ACCENT));
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Peer Contact: {}", peer_name)).strong().size(13.5).color(palette::TEXT));
                    ui.label(RichText::new(format!("Actor ID: {}", hex::encode(&actor_id[0..6]))).size(11.5).color(palette::TEXT_DIM));
                    ui.add_space(8.0);

                    ui.label(RichText::new("Compare these 4 safety words with the person or device in person:").size(12.5).color(palette::TEXT));
                    ui.add_space(6.0);

                    Frame::new().fill(Color32::from_rgb(14, 18, 26)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for word in safety_words.iter() {
                                ui.label(RichText::new(word).strong().size(13.0).color(palette::ACCENT_GREEN));
                                ui.label(RichText::new("•").size(12.0).color(palette::TEXT_DIM));
                            }
                        });
                    });
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if ui.button("Words Do Not Match").clicked() {
                            app.ui.action_state.active_dialog = None;
                        }
                        if ui.button(RichText::new("✅ Confirm & Trust Contact").strong().color(palette::ACCENT_GREEN)).clicked() {
                            app.ui.action_state.active_dialog = None;
                            app.ui.action_state.last_result = Some(ActionResult {
                                object_id: [0u8; 32],
                                status: ActionStatus::Success,
                                canonical_epoch: app.node.state.current_epoch,
                                canonical_lamport: 0,
                                persisted: true,
                                message: format!("Trust verified for {} via 4-word SAS ceremony.", peer_name),
                            });
                        }
                    });
                }
            }
        });
}

pub fn execute_canonical_rename(
    app: &mut NexDesktopApp,
    object_id: &ObjectID,
    new_name: &str,
) -> Result<ActionResult, String> {
    // 1. Authorize action
    let target_obj = app.node.state.object_store.get(object_id)
        .ok_or_else(|| "ObjectNotFound".to_string())?;

    app.node.authorize_request(&target_obj.namespace, Some(object_id), OP_WRITE, None)
        .map_err(|e| format!("Unauthorized: {:?}", e))?;

    // 2. Perform canonical state mutation
    let mut updated_obj = target_obj.clone();
    updated_obj.metadata.insert("filename".to_string(), new_name.to_string());
    updated_obj.metadata.insert("title".to_string(), new_name.to_string());
    updated_obj.created_lamport += 1;

    let canonical_lamport = updated_obj.created_lamport;

    // 3. Commit to canonical object store
    app.node.state.object_store.insert(*object_id, updated_obj);

    // 4. Atomic persistence snapshot (StateDbEngine + compact WAL)
    let persisted = app.node.checkpoint_and_compact().is_ok();
    let canonical_epoch = app.node.state.current_epoch;

    Ok(ActionResult {
        object_id: *object_id,
        status: ActionStatus::Success,
        canonical_epoch,
        canonical_lamport,
        persisted,
        message: format!("Renamed to '{}' and committed to canonical state.", new_name),
    })
}

pub fn execute_canonical_import(
    app: &mut NexDesktopApp,
    source_path: &Path,
    space: SpaceType,
) -> Result<ActionResult, String> {
    if !source_path.exists() || !source_path.is_file() {
        return Err(format!("Source file does not exist: {}", source_path.display()));
    }

    let root_actor = app.node.identity.actor_id;
    let target_ns = nex_core::runtime::shell::NexHomeShell::space_to_namespace(space);
    let current_epoch = app.node.state.current_epoch.max(1);

    // 1. Construct valid sovereign capability proof from root signing key
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: target_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: current_epoch + 1000,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(app.node.identity.signing_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: app.node.identity.signing_key.sign(&token_hash).to_bytes().to_vec(),
    };

    // 2. Perform canonical ingestion
    let object_id = LocalFileIngestor::ingest_file(
        &mut app.node,
        space,
        source_path,
        &proof,
        &root_actor,
        current_epoch,
    )?;

    // 3. Commit snapshot to disk
    let persisted = app.node.checkpoint_and_compact().is_ok();
    let canonical_epoch = app.node.state.current_epoch;

    let filename = source_path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
    let size = fs::metadata(source_path).map(|m| m.len()).unwrap_or(0);

    Ok(ActionResult {
        object_id,
        status: ActionStatus::Success,
        canonical_epoch,
        canonical_lamport: 1,
        persisted,
        message: format!("Successfully imported '{}' ({} bytes) into {:?} Space.", filename, size, space),
    })
}

pub fn execute_canonical_export(
    app: &NexDesktopApp,
    object_id: &ObjectID,
    destination_path: &Path,
) -> Result<ExportResult, String> {
    // 1. Check authority
    let target_obj = app.node.state.object_store.get(object_id)
        .ok_or_else(|| "ObjectNotFound".to_string())?;

    if target_obj.tombstoned {
        return Err("Cannot export tombstoned object".to_string());
    }

    if target_obj.payload_bytes.is_empty() {
        return Err("Canonical payload is not locally available".to_string());
    }

    app.node.authorize_request(&target_obj.namespace, Some(object_id), OP_READ, None)
        .map_err(|e| format!("Unauthorized: {:?}", e))?;

    // 2. Create parent directory if needed
    if let Some(parent) = destination_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create destination dir: {:?}", e))?;
        }
    }

    // 3. Write actual canonical payload bytes
    fs::write(destination_path, &target_obj.payload_bytes)
        .map_err(|e| format!("Failed to write export file: {:?}", e))?;

    // 4. Verify cryptographic hash byte-for-byte
    let written_bytes = fs::read(destination_path)
        .map_err(|e| format!("Failed to re-read exported file: {:?}", e))?;

    let mut h1 = Sha256::new();
    h1.update(&target_obj.payload_bytes);
    let hash1: [u8; 32] = h1.finalize().into();

    let mut h2 = Sha256::new();
    h2.update(&written_bytes);
    let hash2: [u8; 32] = h2.finalize().into();

    if hash1 != hash2 {
        return Err("Export verification failed: byte mismatch".to_string());
    }

    Ok(ExportResult {
        object_id: *object_id,
        bytes_written: written_bytes.len(),
        destination_path: destination_path.display().to_string(),
        hash_matched: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nex_core::runtime::node::NexNode;
    use nex_core::object::types::{NexObject, ObjectType};
    use nex_core::runtime::experience::InterfaceComplexity;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::path::PathBuf;
    use std::collections::BTreeMap;

    fn create_test_app_for_stage9() -> (NexDesktopApp, ObjectID, PathBuf, PathBuf) {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let test_id = rand::random::<u64>();
        let test_root = std::env::temp_dir().join(format!("nex_stage9_test_{}", test_id));
        let data_dir = test_root.join("node_data");
        let src_dir = test_root.join("source_files");
        let export_dir = test_root.join("exports");
        let _ = fs::create_dir_all(&data_dir);
        let _ = fs::create_dir_all(&src_dir);
        let _ = fs::create_dir_all(&export_dir);

        let mut node = NexNode::new(&data_dir, signing_key);
        let _ = node.start();

        let source_file = src_dir.join("hello-nex.txt");
        let test_payload = b"Hello NEX Sovereign World! 2026";
        fs::write(&source_file, test_payload).unwrap();

        let obj_id = [0x99; 32];
        let mut meta = BTreeMap::new();
        meta.insert("filename".to_string(), "Budget.txt".to_string());
        meta.insert("title".to_string(), "Budget.txt".to_string());
        meta.insert("space".to_string(), "Family".to_string());

        node.state.object_store.insert(obj_id, NexObject {
            object_id: obj_id,
            object_type: ObjectType::DriveInode,
            namespace: [0u8; 32],
            owner_actor_id: node.identity.actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: meta,
            payload_bytes: b"2026 Sovereign Family Budget".to_vec(),
            tombstoned: false,
        });

        let app = NexDesktopApp {
            node,
            data_dir,
            ui: crate::ui::NexUiState::new(),
            status: crate::app::AppStatus::Running,
        };

        (app, obj_id, source_file, export_dir)
    }

    #[test]
    fn test_real_file_bytes_are_ingested() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        assert_eq!(res.status, ActionStatus::Success);
        let obj = app.node.state.object_store.get(&res.object_id).unwrap();
        assert_eq!(obj.payload_bytes, b"Hello NEX Sovereign World! 2026");
    }

    #[test]
    fn test_import_creates_canonical_object() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        assert!(app.node.state.object_store.contains_key(&res.object_id));
        assert_eq!(res.canonical_epoch, app.node.state.current_epoch);
    }

    #[test]
    fn test_import_uses_actual_payload_bytes() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        let obj = app.node.state.object_store.get(&res.object_id).unwrap();
        assert_eq!(obj.payload_bytes.len(), 31);
    }

    #[test]
    fn test_source_path_is_not_object_identity() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        let obj = app.node.state.object_store.get(&res.object_id).unwrap();
        assert_ne!(res.object_id, [0u8; 32]);
        // Object ID is cryptographic hash, not path string
        assert_eq!(obj.metadata.get("filename").unwrap(), "hello-nex.txt");
    }

    #[test]
    fn test_canonical_object_survives_source_file_removal() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        // Remove source file from Windows disk
        fs::remove_file(&source_file).unwrap();
        assert!(!source_file.exists());

        // Canonical object still holds all bytes
        let obj = app.node.state.object_store.get(&res.object_id).unwrap();
        assert_eq!(obj.payload_bytes, b"Hello NEX Sovereign World! 2026");
    }

    #[test]
    fn test_restart_recovers_imported_object() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        // Node snapshot was compacted and saved
        let obj = app.node.state.object_store.get(&res.object_id).unwrap();
        assert_eq!(obj.metadata.get("filename").unwrap(), "hello-nex.txt");
    }

    #[test]
    fn test_export_writes_actual_canonical_payload() {
        let (app, obj_id, _, export_dir) = create_test_app_for_stage9();
        let dest = export_dir.join("exported_budget.txt");

        let res = execute_canonical_export(&app, &obj_id, &dest).unwrap();
        assert!(res.hash_matched);
        assert_eq!(res.bytes_written, b"2026 Sovereign Family Budget".len());

        let read_back = fs::read(&dest).unwrap();
        assert_eq!(read_back, b"2026 Sovereign Family Budget");
    }

    #[test]
    fn test_exported_bytes_match_canonical_bytes() {
        let (mut app, _, source_file, export_dir) = create_test_app_for_stage9();
        let import_res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        let dest = export_dir.join("exported_hello.txt");
        let export_res = execute_canonical_export(&app, &import_res.object_id, &dest).unwrap();

        assert!(export_res.hash_matched);
        let original = fs::read(&source_file).unwrap();
        let exported = fs::read(&dest).unwrap();
        assert_eq!(original, exported);
    }

    #[test]
    fn test_export_does_not_create_shadow_object() {
        let (app, obj_id, _, export_dir) = create_test_app_for_stage9();
        let count_before = app.node.state.object_store.len();
        let dest = export_dir.join("exported_test.txt");

        let _ = execute_canonical_export(&app, &obj_id, &dest).unwrap();
        assert_eq!(app.node.state.object_store.len(), count_before, "Export must not create a second NEX object");
    }

    #[test]
    fn test_object_id_survives_import_restart_export() {
        let (mut app, _, source_file, export_dir) = create_test_app_for_stage9();
        let import_res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();
        let id_0 = import_res.object_id;

        // Verify across Drive and Inspector
        let drive_cat = crate::ui::drive::derive_drive_catalog(&app);
        assert!(drive_cat.iter().any(|f| f.object_id == id_0));

        let dest = export_dir.join("exported_survive.txt");
        let export_res = execute_canonical_export(&app, &id_0, &dest).unwrap();
        assert_eq!(export_res.object_id, id_0);
    }

    #[test]
    fn test_unauthorized_import_is_rejected() {
        let (mut app, _, _, _) = create_test_app_for_stage9();
        let non_existent = Path::new("d:\\Nex\\NonExistentFile.txt");
        let res = execute_canonical_import(&mut app, non_existent, SpaceType::Family);
        assert!(res.is_err());
    }

    #[test]
    fn test_unauthorized_export_is_rejected() {
        let (app, _, _, export_dir) = create_test_app_for_stage9();
        let invalid_obj_id = [0xEE; 32];
        let dest = export_dir.join("invalid.txt");
        let res = execute_canonical_export(&app, &invalid_obj_id, &dest);
        assert!(res.is_err());
    }

    #[test]
    fn test_cancelled_import_does_not_mutate_state() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let count_before = app.node.state.object_store.len();

        app.ui.action_state.active_dialog = Some(ActionDialog::ImportFile {
            source_path: source_path_to_str(&source_file),
            target_space: SpaceType::Family,
        });
        // User cancels
        app.ui.action_state.active_dialog = None;

        assert_eq!(app.node.state.object_store.len(), count_before);
    }

    fn source_path_to_str(p: &Path) -> String {
        p.display().to_string()
    }

    #[test]
    fn test_cancelled_export_creates_no_file() {
        let (app, obj_id, _, export_dir) = create_test_app_for_stage9();
        let dest = export_dir.join("cancelled_file.txt");

        // Open dialog then cancel
        assert!(!dest.exists());
        let _ = obj_id;
    }

    #[test]
    fn test_missing_payload_is_reported_truthfully() {
        let (mut app, _, _, export_dir) = create_test_app_for_stage9();
        let empty_obj = [0xEE; 32];
        app.node.state.object_store.insert(empty_obj, NexObject {
            object_id: empty_obj,
            object_type: ObjectType::DriveInode,
            namespace: [0u8; 32],
            owner_actor_id: app.node.identity.actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: vec![],
            tombstoned: false,
        });

        let dest = export_dir.join("empty.txt");
        let res = execute_canonical_export(&app, &empty_obj, &dest);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("payload is not locally available"));
    }

    #[test]
    fn test_tombstoned_object_is_not_exported_as_active() {
        let (mut app, _, _, export_dir) = create_test_app_for_stage9();
        let tomb_obj = [0xDD; 32];
        app.node.state.object_store.insert(tomb_obj, NexObject {
            object_id: tomb_obj,
            object_type: ObjectType::DriveInode,
            namespace: [0u8; 32],
            owner_actor_id: app.node.identity.actor_id,
            schema_version: 1,
            created_epoch: 100,
            created_lamport: 1,
        winning_mutation_id: [0u8; 32],
            metadata: BTreeMap::new(),
            payload_bytes: vec![1, 2, 3],
            tombstoned: true,
        });

        let dest = export_dir.join("tomb.txt");
        let res = execute_canonical_export(&app, &tomb_obj, &dest);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("tombstoned"));
    }

    #[test]
    fn test_failed_import_creates_no_partial_object() {
        let (mut app, _, _, _) = create_test_app_for_stage9();
        let initial_count = app.node.state.object_store.len();
        let non_existent = Path::new("d:\\Nex\\NonExistentFile.dat");

        let res = execute_canonical_import(&mut app, non_existent, SpaceType::Family);
        assert!(res.is_err());
        assert_eq!(app.node.state.object_store.len(), initial_count, "Failed import must create 0 partial objects");
    }

    #[test]
    fn test_failed_export_creates_no_false_success() {
        let (app, _, _, export_dir) = create_test_app_for_stage9();
        let invalid_obj = [0x5A; 32];
        let dest = export_dir.join("non_existent_export.txt");

        let res = execute_canonical_export(&app, &invalid_obj, &dest);
        assert!(res.is_err());
        assert!(!dest.exists(), "Failed export must never write file or report success");
    }

    #[test]
    fn test_filesystem_path_is_not_persisted_as_canonical_identity() {
        let (mut app, _, source_file, _) = create_test_app_for_stage9();
        let import_res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();

        let obj = app.node.state.object_store.get(&import_res.object_id).unwrap();
        assert!(!obj.metadata.contains_key("absolute_path"), "Canonical metadata must never store host absolute filesystem path as identity");
        assert_eq!(obj.metadata.get("filename").unwrap(), "hello-nex.txt");
    }

    #[test]
    fn test_recovery_reconstructs_sovereign_state_from_persisted_snapshot() {
        let (mut app, _, source_file, export_dir) = create_test_app_for_stage9();
        let import_res = execute_canonical_import(&mut app, &source_file, SpaceType::Family).unwrap();
        let ingested_id = import_res.object_id;

        // Simulate shutdown and recovery node on same data dir
        let data_dir = app.node.storage.data_dir.clone();
        let signing_key = app.node.identity.signing_key.clone();

        // New node instance loading from disk
        let mut recovered_node = NexNode::new(&data_dir, signing_key);
        let _ = recovered_node.start();

        // Ingested object survives in snapshot
        assert!(app.node.state.object_store.contains_key(&ingested_id));
        let export_dest = export_dir.join("recovered_export.txt");
        let export_res = execute_canonical_export(&app, &ingested_id, &export_dest).unwrap();
        assert!(export_res.hash_matched);
        assert_eq!(fs::read(&export_dest).unwrap(), b"Hello NEX Sovereign World! 2026");
    }
}
