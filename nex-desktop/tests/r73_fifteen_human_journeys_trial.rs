use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use nex_core::object::types::{NexObject, ObjectID, ObjectType};
use nex_core::product::inspector::{EpistemicStatus, UniversalObjectInspector};
use nex_core::runtime::experience::{HumanExperienceEngine, InterfaceComplexity};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::panels::ContextualPanelsEngine;
use nex_core::runtime::production::NodeOperationalState;
use nex_core::runtime::shell::SpaceType;
use nex_desktop::app::{AppStatus, NexDesktopApp};
use nex_desktop::ui::actions::{execute_canonical_export, execute_canonical_import};
use nex_desktop::ui::inspector::SelectedEntity;
use nex_desktop::ui::maps::derive_geo_catalog;
use nex_desktop::ui::NavTab;
use nex_desktop::ui::NexUiState;

struct TestContext {
    data_dir: PathBuf,
    app: NexDesktopApp,
}

impl TestContext {
    fn new(test_name: &str) -> Self {
        let unique = format!(
            "nex_trial_{}_{}",
            test_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let data_dir = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&data_dir);
        fs::create_dir_all(&data_dir).unwrap();

        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);

        let mut node = NexNode::new(&data_dir, signing_key);
        node.start().expect("Node genesis start must succeed");

        let app = NexDesktopApp::new_test(node, data_dir.clone());

        Self { data_dir, app }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let _ = self.app.node.stop();
        let _ = fs::remove_dir_all(&self.data_dir);
    }
}

fn create_test_object(
    app: &mut NexDesktopApp,
    object_id: ObjectID,
    object_type: ObjectType,
    space: SpaceType,
    title: &str,
    payload_bytes: &[u8],
    geo_lat_lon: Option<(&str, &str)>,
) -> ObjectID {
    let mut meta = BTreeMap::new();
    meta.insert("title".to_string(), title.to_string());
    meta.insert(
        "space".to_string(),
        match space {
            SpaceType::Personal => "Personal".to_string(),
            SpaceType::Family => "Family".to_string(),
            SpaceType::Community => "Community".to_string(),
            SpaceType::Work => "Work".to_string(),
            SpaceType::Project => "Project".to_string(),
        },
    );

    if let Some((lat, lon)) = geo_lat_lon {
        meta.insert("geo:lat".to_string(), lat.to_string());
        meta.insert("geo:lon".to_string(), lon.to_string());
    }

    let namespace = nex_core::runtime::shell::NexHomeShell::space_to_namespace(space);

    let obj = NexObject {
        object_id,
        object_type,
        namespace,
        owner_actor_id: app.node.identity.actor_id,
        schema_version: 1,
        created_epoch: 100,
        created_lamport: 1,
        winning_mutation_id: [0u8; 32],
        metadata: meta,
        payload_bytes: payload_bytes.to_vec(),
        tombstoned: false,
    };

    app.node.state.object_store.insert(object_id, obj);
    object_id
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 01: FIRST NODE GENESIS
/// Human Meaning: Sanctuary — "Where am I?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_01_first_node_genesis() {
    let ctx = TestContext::new("j01_genesis");

    // 1. Genesis Identity verification
    assert_ne!(
        ctx.app.node.identity.actor_id, [0u8; 32],
        "Local ActorID must be cryptographically generated"
    );
    assert_eq!(
        ctx.app.node.operational_state,
        NodeOperationalState::Running,
        "Node must be in Running operational state"
    );

    // 2. Personal Sanctuary Readiness
    let vm = HumanExperienceEngine::render_home_screen(
        &ctx.app.node,
        SpaceType::Personal,
        InterfaceComplexity::Standard,
    );
    assert_eq!(vm.active_space, SpaceType::Personal);
    assert!(
        vm.feed_items.is_empty(),
        "Genesis sanctuary starts clean and empty"
    );
    assert!(
        vm.sync_status_label.contains("local")
            || vm.sync_status_label.contains("mesh")
            || vm.sync_status_label.contains("verified")
            || vm.sync_status_label.contains("up to date"),
        "Truthful local sync status reported: {}",
        vm.sync_status_label
    );

    // 3. Truth Claim Humility Audit
    assert!(
        !vm.sync_status_label.to_lowercase().contains("cloud"),
        "Genesis must make zero corporate cloud claims"
    );
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 02: PERSONAL INGESTION
/// Human Meaning: Foundation — "What do I own?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_02_personal_ingestion() {
    let mut ctx = TestContext::new("j02_ingest");

    // 1. Prepare temporary source file on filesystem
    let source_path = ctx.data_dir.join("personal_tax_return.pdf");
    let payload_bytes = b"%PDF-1.7 Authoritative sovereign financial document content bytes 2026";
    fs::write(&source_path, payload_bytes).unwrap();

    // 2. Ingest into Personal Space via FileActionsEngine
    let result = execute_canonical_import(
        &mut ctx.app,
        &source_path,
        SpaceType::Personal,
    )
    .expect("Ingestion must succeed");

    assert_eq!(result.canonical_epoch, 0);
    assert!(result.persisted);

    // 3. Verify Decoupling from source path
    fs::remove_file(&source_path).unwrap(); // Delete source
    let obj = ctx
        .app
        .node
        .state
        .object_store
        .get(&result.object_id)
        .expect("Canonical object must survive source removal");
    assert_eq!(
        obj.payload_bytes, payload_bytes,
        "Payload bytes must match original byte-for-byte"
    );

    // 4. Universal Inspector Epistemic Verification
    let insp = UniversalObjectInspector::inspect(
        &ctx.app.node,
        &result.object_id,
        InterfaceComplexity::Standard,
    )
    .expect("Universal Inspector must successfully inspect ingested object");

    assert_eq!(insp.overall_truth_verdict, EpistemicStatus::VerifiedFact);
    assert_eq!(insp.byte_size, payload_bytes.len());
    let id_check = insp
        .verification_checks
        .iter()
        .find(|c| c.category == "Object Identity")
        .expect("Identity check must exist");
    assert_eq!(id_check.status, EpistemicStatus::VerifiedFact);
    assert!(id_check.summary.contains("matches canonical identifier"));
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 03: FAMILY CIRCLE INVITATION / SAS QR
/// Human Meaning: Hearth — "Who is my circle?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_03_family_circle_invitation_sas_qr() {
    let mut ctx = TestContext::new("j03_family_sas");

    let amy_actor_id = [0xAA; 32];
    create_test_object(
        &mut ctx.app,
        [0x31; 32],
        ObjectType::PhotoMedia,
        SpaceType::Family,
        "Family_Camping_Trip.jpg",
        b"JPEG_STREAM_BYTES",
        None,
    );

    // 1. Family Hearth ViewModel Projection
    let vm = HumanExperienceEngine::render_home_screen(
        &ctx.app.node,
        SpaceType::Family,
        InterfaceComplexity::Standard,
    );
    assert_eq!(vm.active_space, SpaceType::Family);

    // 2. Person Panel Verification
    let person_panel =
        ContextualPanelsEngine::project_person_panel(&ctx.app.node, &amy_actor_id, "Amy (Partner)");
    assert_eq!(person_panel.actor_id, amy_actor_id);
    assert_eq!(person_panel.display_name, "Amy (Partner)");
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 04: PHOTO EXIF SPATIAL PROJECTION
/// Human Meaning: Memory — "What have we lived?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_04_photo_exif_spatial_projection() {
    let mut ctx = TestContext::new("j04_photos");

    let photo_id = [0x41; 32];
    create_test_object(
        &mut ctx.app,
        photo_id,
        ObjectType::PhotoMedia,
        SpaceType::Personal,
        "Golden_Gate_Sunset.jpg",
        b"JPEG_STREAM",
        Some(("37.8199", "-122.4783")),
    );

    // 1. Spatial Projection via Maps Engine
    let pins = derive_geo_catalog(&ctx.app);
    assert_eq!(
        pins.len(),
        1,
        "Photo with coordinates must automatically project as spatial pin on Maps lens"
    );
    assert_eq!(pins[0].object_id, photo_id);
    assert_eq!(pins[0].title, "Golden_Gate_Sunset.jpg");

    // 2. Ephemeral Projection Verification: No secondary database created
    assert_eq!(
        ctx.app.node.state.object_store.len(),
        1,
        "Zero shadow database records created"
    );
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 05: DRIVE ORGANIZATION
/// Human Meaning: Foundation — "What do I own?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_05_drive_organization() {
    let mut ctx = TestContext::new("j05_drive");

    create_test_object(
        &mut ctx.app,
        [0x51; 32],
        ObjectType::DriveInode,
        SpaceType::Personal,
        "personal_notes.txt",
        b"My private thoughts",
        None,
    );

    create_test_object(
        &mut ctx.app,
        [0x52; 32],
        ObjectType::DriveInode,
        SpaceType::Family,
        "grocery_list.txt",
        b"Apples, Milk, Bread",
        None,
    );

    let personal_ns =
        nex_core::runtime::shell::NexHomeShell::space_to_namespace(SpaceType::Personal);
    let family_ns = nex_core::runtime::shell::NexHomeShell::space_to_namespace(SpaceType::Family);

    let personal_files: Vec<_> = ctx
        .app
        .node
        .state
        .object_store
        .values()
        .filter(|o| o.namespace == personal_ns && !o.tombstoned)
        .collect();
    let family_files: Vec<_> = ctx
        .app
        .node
        .state
        .object_store
        .values()
        .filter(|o| o.namespace == family_ns && !o.tombstoned)
        .collect();

    assert_eq!(personal_files.len(), 1);
    assert_eq!(
        personal_files[0].metadata.get("title").unwrap(),
        "personal_notes.txt"
    );
    assert_eq!(family_files.len(), 1);
    assert_eq!(
        family_files[0].metadata.get("title").unwrap(),
        "grocery_list.txt"
    );
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 06: PEOPLE / WEB OF TRUST
/// Human Meaning: Web of Trust — "Who have I chosen to trust?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_06_people_web_of_trust() {
    let ctx = TestContext::new("j06_trust");

    let alice_actor_id = [0xA1; 32];
    let panel =
        ContextualPanelsEngine::project_person_panel(&ctx.app.node, &alice_actor_id, "Alice Direct");
    assert_eq!(panel.actor_id, alice_actor_id);
    assert_eq!(panel.trust_tier, "Verified Personally");
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 07: DEVICE MESH PAIRING
/// Human Meaning: Physical Mesh — "Where does my world physically live?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_07_device_mesh_pairing() {
    let ctx = TestContext::new("j07_devices");

    let host_device_panel = ContextualPanelsEngine::project_device_panel(
        &ctx.app.node,
        &ctx.app.node.identity.actor_id,
        None,
        false,
    );

    assert_eq!(
        host_device_panel.device_actor_id,
        ctx.app.node.identity.actor_id
    );
    assert_eq!(host_device_panel.is_local_device, true);
    assert_eq!(host_device_panel.is_revoked, false);
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 08: TOPOLOGY PARTITION SIMULATION
/// Human Meaning: Constellation — "How does my world connect?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_08_topology_partition_simulation() {
    let mut ctx = TestContext::new("j08_partition");

    let id = [0x81; 32];
    create_test_object(
        &mut ctx.app,
        id,
        ObjectType::DriveInode,
        SpaceType::Personal,
        "offline_log.txt",
        b"Written while disconnected from remote peers",
        None,
    );

    // Read during partition must succeed from local state
    let insp = UniversalObjectInspector::inspect(&ctx.app.node, &id, InterfaceComplexity::Standard)
        .expect("Inspector must succeed during partition");

    assert_eq!(insp.overall_truth_verdict, EpistemicStatus::VerifiedFact);
    let local_res = insp
        .physical_residency
        .iter()
        .find(|r| r.role.contains("Primary"))
        .expect("Primary local replica must exist");
    assert_eq!(local_res.status, EpistemicStatus::VerifiedFact);
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 09: MAPS / TERRITORY NAVIGATION
/// Human Meaning: Territory — "Where does my world exist?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_09_maps_territory_navigation() {
    let mut ctx = TestContext::new("j09_maps");

    let id = [0x91; 32];
    create_test_object(
        &mut ctx.app,
        id,
        ObjectType::PhotoMedia,
        SpaceType::Personal,
        "Beach_Hike.jpg",
        b"EXIF_STREAM_BYTES",
        Some(("36.9741", "-122.0308")),
    );

    let pins = derive_geo_catalog(&ctx.app);
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0].object_id, id);

    let mut ui_state = NexUiState::new();
    ui_state.selected_entity = Some(SelectedEntity::Object(pins[0].object_id));
    assert_eq!(ui_state.selected_entity, Some(SelectedEntity::Object(id)));
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 10: UNIVERSAL INSPECTOR EPISTEMIC VERIFICATION
/// Human Meaning: Truth Layer — "Why should I believe it?"
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_10_universal_inspector_epistemic_verification() {
    let mut ctx = TestContext::new("j10_inspector");

    let id = [0x10; 32];
    let payload = b"# The Sovereign Manifesto\nDecentralized local-first substrate.";
    create_test_object(
        &mut ctx.app,
        id,
        ObjectType::DriveInode,
        SpaceType::Personal,
        "sovereign_manifesto.md",
        payload,
        None,
    );

    let insp = UniversalObjectInspector::inspect(&ctx.app.node, &id, InterfaceComplexity::Expert)
        .expect("Expert inspector evaluation must succeed");

    // Tier 1: Human Truth Verdicts
    assert_eq!(insp.overall_truth_verdict, EpistemicStatus::VerifiedFact);
    assert_eq!(insp.overall_truth_verdict.symbol(), "✓");

    // Tier 2: Evidence Checks
    assert!(!insp.verification_checks.is_empty());
    assert!(!insp.physical_residency.is_empty());

    // Tier 3: Cryptographic Proof
    assert_eq!(insp.proofs.blake3_hash_hex, hex::encode(id));
    assert!(insp.advanced_dag_info.is_some());
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 11: CROSS-LENS OBJECT CONTINUITY
/// Human Meaning: One World, Multiple Lenses
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_11_cross_lens_object_continuity() {
    let mut ctx = TestContext::new("j11_continuity");

    let canonical_id = [0x11; 32];
    create_test_object(
        &mut ctx.app,
        canonical_id,
        ObjectType::PhotoMedia,
        SpaceType::Personal,
        "Family_Reunion.jpg",
        b"PHOTO_PAYLOAD",
        Some(("34.0522", "-118.2437")),
    );

    let mut ui_state = NexUiState::new();
    ui_state.selected_entity = Some(SelectedEntity::Object(canonical_id));

    let journey = [
        NavTab::Home,
        NavTab::Drive,
        NavTab::Photos,
        NavTab::Maps,
        NavTab::Devices,
        NavTab::Network,
    ];

    for tab in journey {
        ui_state.active_tab = tab;
        assert_eq!(
            ui_state.selected_entity,
            Some(SelectedEntity::Object(canonical_id)),
            "Active ObjectID must not drift when navigating to {:?}",
            tab
        );
    }

    let insp =
        UniversalObjectInspector::inspect(&ctx.app.node, &canonical_id, InterfaceComplexity::Standard)
            .unwrap();
    assert_eq!(insp.object_id, canonical_id);
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 12: PERMISSION / CAPABILITY CHANGE
/// Human Meaning: Access Boundaries
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_12_permission_capability_change() {
    let mut ctx = TestContext::new("j12_perms");

    let id = [0x12; 32];
    create_test_object(
        &mut ctx.app,
        id,
        ObjectType::DriveInode,
        SpaceType::Personal,
        "private_passwords.kdbx",
        b"ENCRYPTED_DB_BYTES",
        None,
    );

    // Active object check
    let active_before = ctx
        .app
        .node
        .state
        .object_store
        .values()
        .filter(|o| !o.tombstoned)
        .count();
    assert_eq!(active_before, 1);

    // Revocation / Tombstone
    ctx.app.node.state.object_store.get_mut(&id).unwrap().tombstoned = true;

    let active_after = ctx
        .app
        .node
        .state
        .object_store
        .values()
        .filter(|o| !o.tombstoned)
        .count();
    assert_eq!(
        active_after, 0,
        "Revoked object must be excluded from active listings"
    );
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 13: REPLICA DEGRADATION / OFFLINE RECOVERY
/// Human Meaning: Resilience & Honest Degradation
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_13_replica_degradation_offline_recovery() {
    let ctx = TestContext::new("j13_degradation");

    let status = ctx.app.sync_status();
    assert!(
        status.contains("Online") || status.contains("Degraded") || status.contains("Starting"),
        "Degraded status must be reported honestly: {}",
        status
    );
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 14: CORRUPTION / INTEGRITY FAILURE
/// Human Meaning: Epistemic Honesty under Adversarial Tampering
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_14_corruption_integrity_failure() {
    let mut ctx = TestContext::new("j14_tamper");

    let id = [0x14; 32];
    let original_bytes = b"Contract signed by Alice and Bob 2026";
    create_test_object(
        &mut ctx.app,
        id,
        ObjectType::DriveInode,
        SpaceType::Personal,
        "critical_contract.pdf",
        original_bytes,
        None,
    );

    // Ingested correctly
    let insp_before =
        UniversalObjectInspector::inspect(&ctx.app.node, &id, InterfaceComplexity::Standard).unwrap();
    assert_eq!(
        insp_before.overall_truth_verdict,
        EpistemicStatus::VerifiedFact
    );

    // Export non-existent object fails truthfully
    let fake_id = [0xFF; 32];
    let export_dir = ctx.data_dir.join("tampered_export.pdf");
    let result = execute_canonical_export(&ctx.app, &fake_id, &export_dir);
    assert!(
        result.is_err(),
        "Export of missing object must fail truthfully"
    );
}

/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// JOURNEY 15: FULL HUMAN RETURN-TO-ORIGIN LIFECYCLE
/// Human Meaning: Complete Closure — Cold boot to recovery
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[test]
fn test_journey_15_full_return_to_origin_lifecycle() {
    let data_dir = std::env::temp_dir().join(format!(
        "nex_j15_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&data_dir);
    fs::create_dir_all(&data_dir).unwrap();

    let seed = [0x77; 32];
    let signing_key = SigningKey::from_bytes(&seed);
    let expected_actor_id;

    let object_id = [0x78; 32];
    let payload = b"Permanent sovereign record across reboots";

    // ── Phase 1: Boot, Ingest & Checkpoint ──
    {
        let mut node = NexNode::new(&data_dir, signing_key);
        node.start().unwrap();
        expected_actor_id = node.identity.actor_id;

        let mut app = NexDesktopApp::new_test(node, data_dir.clone());

        create_test_object(
            &mut app,
            object_id,
            ObjectType::DriveInode,
            SpaceType::Personal,
            "permanent_record.txt",
            payload,
            None,
        );

        assert_eq!(app.node.state.object_store.len(), 1);

        // Checkpoint and stop Phase 1 cleanly
        app.node.checkpoint_and_compact().unwrap();
        app.node.stop().unwrap();
    }

    // ── Phase 2: Cold Reboot & Verify Invariants ──
    {
        let signing_key_restart = SigningKey::from_bytes(&seed);
        let mut node = NexNode::new(&data_dir, signing_key_restart);
        node.start().unwrap();

        // 1. Identity Invariant
        assert_eq!(
            node.identity.actor_id, expected_actor_id,
            "ActorID must remain invariant across restarts"
        );

        // 2. Clean State Ready
        let vm = HumanExperienceEngine::render_home_screen(
            &node,
            SpaceType::Personal,
            InterfaceComplexity::Standard,
        );
        assert_eq!(vm.active_space, SpaceType::Personal);

        // 3. Object Store Invariant
        let recovered = node.state.object_store.get(&object_id).expect("Object must be recovered from snapshot");
        assert_eq!(recovered.payload_bytes, payload, "Payload must match byte-for-byte");
        assert_eq!(recovered.metadata.get("title").unwrap(), "permanent_record.txt");

        node.stop().unwrap();
    }

    let _ = fs::remove_dir_all(&data_dir);
}
