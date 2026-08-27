use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

use nex_core::runtime::node::NexNode;
use nex_core::runtime::experience::InterfaceComplexity;
use nex_core::runtime::shell::SpaceType;
use nex_core::object::types::{NexObject, ObjectType};

use nex_desktop::app::{NexDesktopApp, AppStatus};
use nex_desktop::ui::{NavTab, NexUiState, inspector, actions};
use nex_desktop::ui::palette_command::{CommandPaletteState, PaletteCategory, PaletteActionPayload, CommandActionType};

fn setup_test_app() -> NexDesktopApp {
    let tmp = tempdir().unwrap();
    let mut csprng = OsRng;
    let key = SigningKey::generate(&mut csprng);
    let mut node = NexNode::new(tmp.path(), key);
    node.start().unwrap();

    NexDesktopApp {
        node,
        data_dir: tmp.into_path(),
        ui: NexUiState::new(),
        status: AppStatus::Running,
    }
}

#[test]
fn test_palette_toggle_and_reset() {
    let mut app = setup_test_app();
    let mut palette_state = CommandPaletteState::new();

    assert!(!app.ui.command_palette_open);

    // Open palette
    app.ui.command_palette_open = true;
    palette_state.query = "random filter".to_string();
    palette_state.selected_index = 3;

    // Reset palette
    palette_state.reset();
    assert_eq!(palette_state.query, "");
    assert_eq!(palette_state.selected_index, 0);
}

#[test]
fn test_palette_arrow_navigation_and_wrapping() {
    let app = setup_test_app();
    let mut palette_state = CommandPaletteState::new();

    let items = palette_state.build_items(&app);
    let total = items.len();
    assert!(total >= 15, "Default palette must have lens, space, complexity, and action items");

    // Moving down wraps cyclically
    palette_state.selected_index = 0;
    palette_state.selected_index = (palette_state.selected_index + 1) % total;
    assert_eq!(palette_state.selected_index, 1);

    // Moving up from 0 wraps to total - 1
    palette_state.selected_index = 0;
    if palette_state.selected_index == 0 {
        palette_state.selected_index = total - 1;
    }
    assert_eq!(palette_state.selected_index, total - 1);
}

#[test]
fn test_palette_9_surfaces_filtering_and_instant_navigation() {
    let mut app = setup_test_app();
    let mut palette_state = CommandPaletteState::new();

    let expected_lenses = [
        ("personal", NavTab::Home),
        ("family", NavTab::Family),
        ("photos", NavTab::Photos),
        ("drive", NavTab::Drive),
        ("people", NavTab::People),
        ("devices", NavTab::Devices),
        ("topology", NavTab::Network),
        ("maps", NavTab::Maps),
        ("settings", NavTab::Settings),
    ];

    for (query, expected_tab) in expected_lenses {
        palette_state.query = query.to_string();
        let items = palette_state.build_items(&app);
        assert!(!items.is_empty(), "Query '{}' must return at least one item", query);

        let match_item = items.iter().find(|i| matches!(i.payload, PaletteActionPayload::Navigate(tab) if tab == expected_tab));
        assert!(match_item.is_some(), "Query '{}' must match lens {:?}", query, expected_tab);

        // Execute navigation
        palette_state.execute_item(match_item.unwrap(), &mut app);
        assert_eq!(app.ui.active_tab, expected_tab);
        assert!(!app.ui.command_palette_open, "Palette must close upon navigation");
    }
}

#[test]
fn test_palette_space_switching() {
    let mut app = setup_test_app();
    let mut palette_state = CommandPaletteState::new();

    palette_state.query = "space".to_string();
    let items = palette_state.build_items(&app);

    let space_items: Vec<_> = items.into_iter().filter(|i| i.category == PaletteCategory::Space).collect();
    assert_eq!(space_items.len(), 4, "Must expose Personal, Family, Work, Community spaces");

    // Switch to Family
    let family_item = space_items.iter().find(|i| matches!(i.payload, PaletteActionPayload::SwitchSpace(SpaceType::Family))).unwrap();
    palette_state.execute_item(family_item, &mut app);
    assert_eq!(app.ui.active_tab, NavTab::Family);

    // Switch to Personal
    let personal_item = space_items.iter().find(|i| matches!(i.payload, PaletteActionPayload::SwitchSpace(SpaceType::Personal))).unwrap();
    palette_state.execute_item(personal_item, &mut app);
    assert_eq!(app.ui.active_tab, NavTab::Home);
}

#[test]
fn test_palette_complexity_slider_switching() {
    let mut app = setup_test_app();
    let mut palette_state = CommandPaletteState::new();

    assert_eq!(app.ui.complexity, InterfaceComplexity::Standard);

    palette_state.query = "expert".to_string();
    let items = palette_state.build_items(&app);
    let expert_item = items.iter().find(|i| matches!(i.payload, PaletteActionPayload::SetComplexity(InterfaceComplexity::Expert))).unwrap();

    palette_state.execute_item(expert_item, &mut app);
    assert_eq!(app.ui.complexity, InterfaceComplexity::Expert);

    palette_state.query = "simple".to_string();
    let items = palette_state.build_items(&app);
    let simple_item = items.iter().find(|i| matches!(i.payload, PaletteActionPayload::SetComplexity(InterfaceComplexity::Simple))).unwrap();

    palette_state.execute_item(simple_item, &mut app);
    assert_eq!(app.ui.complexity, InterfaceComplexity::Simple);
}

#[test]
fn test_palette_live_canonical_object_search_and_inspect() {
    let mut app = setup_test_app();
    let mut palette_state = CommandPaletteState::new();

    // Insert canonical objects into live node state
    let photo_id = [0x11; 32];
    let mut photo_meta = BTreeMap::new();
    photo_meta.insert("title".to_string(), "Alps Summit Panorama".to_string());
    photo_meta.insert("mime".to_string(), "image/jpeg".to_string());
    photo_meta.insert("camera_make".to_string(), "Sony Alpha".to_string());

    app.node.state.object_store.insert(photo_id, NexObject {
        object_id: photo_id,
        namespace: [0xAA; 32],
        object_type: ObjectType::PhotoMedia,
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 10,
        owner_actor_id: app.node.identity.actor_id,
        winning_mutation_id: [0u8; 32],
        metadata: photo_meta,
        payload_bytes: vec![0xEE; 2048],
        tombstoned: false,
    });

    let doc_id = [0x22; 32];
    let mut doc_meta = BTreeMap::new();
    doc_meta.insert("filename".to_string(), "Quarterly_Sovereign_Audit.pdf".to_string());
    doc_meta.insert("mime".to_string(), "application/pdf".to_string());

    app.node.state.object_store.insert(doc_id, NexObject {
        object_id: doc_id,
        namespace: [0xAA; 32],
        object_type: ObjectType::DriveInode,
        schema_version: 1,
        created_epoch: 1,
        created_lamport: 11,
        owner_actor_id: app.node.identity.actor_id,
        winning_mutation_id: [0u8; 32],
        metadata: doc_meta,
        payload_bytes: vec![0xCC; 4096],
        tombstoned: false,
    });

    // Search by title "Alps"
    palette_state.query = "alps".to_string();
    let items = palette_state.build_items(&app);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Alps Summit Panorama");
    assert_eq!(items[0].category, PaletteCategory::Object);

    // Execute inspect Alps
    palette_state.execute_item(&items[0], &mut app);
    assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(photo_id)));

    // Search by filename "Audit"
    palette_state.query = "audit".to_string();
    let items = palette_state.build_items(&app);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Quarterly_Sovereign_Audit.pdf");

    // Execute inspect Audit
    palette_state.execute_item(&items[0], &mut app);
    assert_eq!(app.ui.selected_entity, Some(inspector::SelectedEntity::Object(doc_id)));
}

#[test]
fn test_palette_sovereign_actions_trigger() {
    let mut app = setup_test_app();
    let mut palette_state = CommandPaletteState::new();

    // Trigger SAS Pairing
    palette_state.query = "pair".to_string();
    let items = palette_state.build_items(&app);
    let pair_item = items.iter().find(|i| matches!(i.payload, PaletteActionPayload::TriggerAction(CommandActionType::ProximitySasPairing))).unwrap();

    palette_state.execute_item(pair_item, &mut app);
    assert!(matches!(app.ui.action_state.active_dialog, Some(actions::ActionDialog::ProximitySasVerification { .. })));

    // Trigger Integrity Verification
    palette_state.query = "integrity".to_string();
    let items = palette_state.build_items(&app);
    let verify_item = items.iter().find(|i| matches!(i.payload, PaletteActionPayload::TriggerAction(CommandActionType::VerifyIntegrity))).unwrap();

    palette_state.execute_item(verify_item, &mut app);
    assert!(palette_state.last_executed_feedback.as_ref().unwrap().contains("BLAKE3"));
}
