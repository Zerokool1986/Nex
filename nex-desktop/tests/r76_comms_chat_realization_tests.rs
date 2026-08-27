use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

use nex_core::runtime::node::NexNode;
use nex_core::object::types::{NexObject, ObjectType};
use nex_desktop::app::NexDesktopApp;
use nex_desktop::ui::{NavTab, inspector::SelectedEntity};
use nex_desktop::ui::chat::{derive_chat_channels, derive_channel_messages};
use nex_desktop::ui::palette_command::{CommandPaletteState, PaletteActionPayload};

fn setup_chat_test_app() -> (NexDesktopApp, tempfile::TempDir) {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    let app = NexDesktopApp::new_test(node, tmp.path().to_path_buf());
    (app, tmp)
}

#[test]
fn test_comms_channel_roster_and_direct_chat_projection() {
    let (mut app, _tmp) = setup_chat_test_app();

    // 1. Ingest explicit canonical channel
    let chan_id = [0x77; 32];
    let mut meta = BTreeMap::new();
    meta.insert("name".to_string(), "Project Sovereign Mesh".to_string());
    meta.insert("space".to_string(), "Work".to_string());
    meta.insert("is_direct".to_string(), "false".to_string());

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

    let channels = derive_chat_channels(&app);
    assert!(!channels.is_empty());

    let proj_chan = channels.iter().find(|c| c.channel_id == chan_id);
    assert!(proj_chan.is_some(), "Explicit channel must be projected");
    assert_eq!(proj_chan.unwrap().name, "Project Sovereign Mesh");
    assert_eq!(proj_chan.unwrap().space_name, "Work");
}

#[test]
fn test_comms_message_thread_derivation_and_e2ee_status() {
    let (mut app, _tmp) = setup_chat_test_app();

    let chan_id = [0x88; 32];
    let msg_id = [0x99; 32];

    let mut meta = BTreeMap::new();
    meta.insert("channel_id".to_string(), hex::encode(chan_id));
    meta.insert("space".to_string(), "Family".to_string());
    meta.insert("author_name".to_string(), "Amy".to_string());

    app.node.state.object_store.insert(msg_id, NexObject {
        object_id: msg_id,
        namespace: [0xCA; 32],
        object_type: ObjectType::ChatMessage,
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 5,
        owner_actor_id: [0x55; 32],
        winning_mutation_id: [0u8; 32],
        metadata: meta,
        payload_bytes: b"Secret family recipe attached!".to_vec(),
        tombstoned: false,
    });

    let messages = derive_channel_messages(&app, chan_id);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].plaintext, "Secret family recipe attached!");
    assert_eq!(messages[0].author_name, "Amy");
    assert!(messages[0].e2ee_verified, "Message must have verified E2EE tag");
}

#[test]
fn test_comms_attachment_continuity_to_universal_inspector() {
    let (mut app, _tmp) = setup_chat_test_app();

    let photo_id = [0xFA; 32];
    let mut photo_meta = BTreeMap::new();
    photo_meta.insert("title".to_string(), "Summit Sunset".to_string());
    photo_meta.insert("space".to_string(), "Family".to_string());

    app.node.state.object_store.insert(photo_id, NexObject {
        object_id: photo_id,
        namespace: [0xFA; 32],
        object_type: ObjectType::PhotoMedia,
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 2,
        owner_actor_id: app.node.identity.actor_id,
        winning_mutation_id: [0u8; 32],
        metadata: photo_meta,
        payload_bytes: vec![0x12; 1024],
        tombstoned: false,
    });

    let chan_id = [0xAA; 32];
    let messages = derive_channel_messages(&app, chan_id);

    assert!(!messages.is_empty());
    let msg_with_att = messages.iter().find(|m| !m.attachments.is_empty()).unwrap();
    let attached_id = msg_with_att.attachments[0];

    // Verify navigating to Inspector preserves identical ObjectID
    app.ui.selected_entity = Some(SelectedEntity::Object(attached_id));
    app.ui.active_tab = NavTab::Inspector;

    assert_eq!(app.ui.selected_entity, Some(SelectedEntity::Object(photo_id)));
    assert_eq!(app.ui.active_tab, NavTab::Inspector);
}

#[test]
fn test_comms_command_palette_navigation() {
    let (app, _tmp) = setup_chat_test_app();
    let mut palette_state = CommandPaletteState::new();

    palette_state.query = "chat".to_string();
    let items = palette_state.build_items(&app);

    let chat_item = items.iter().find(|i| i.title.contains("Chat"));
    assert!(chat_item.is_some(), "Command Palette must find NEX Chat lens");

    if let PaletteActionPayload::Navigate(tab) = chat_item.unwrap().payload {
        assert_eq!(tab, NavTab::Chat);
    } else {
        panic!("Chat item payload must be NavTab::Chat");
    }
}
