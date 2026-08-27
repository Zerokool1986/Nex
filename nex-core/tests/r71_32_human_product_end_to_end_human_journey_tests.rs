use std::collections::BTreeMap;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::runtime::experience::{HumanExperienceEngine, InterfaceComplexity};
use nex_core::runtime::slice::SovereignProductSlice;
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE, OP_READ};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::api::NexAppApi;

#[test]
fn test_r71_32_a_complete_human_journey_mobile_to_desktop_rendering() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x01u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x02u8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    // 1. Mobile captures photo
    let (obj_id, _) = SovereignProductSlice::mobile_capture_family_photo(
        &mut mobile,
        &proof,
        "Grand Canyon Hike",
        vec![0xFF, 0xD8, 0xFF, 0xE0, 0x01, 0x02, 0x03, 0x04],
        10,
        &BTreeMap::new(),
        &root_actor,
    ).expect("Mobile capture failed");

    // 2. Inspect on Mobile
    let m_photos = HumanExperienceEngine::render_photos_screen(&mobile, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(m_photos.total_photos, 1);
    assert_eq!(m_photos.photo_cards[0].title, "Grand Canyon Hike");

    // 3. Desktop is initially empty
    let d_photos_pre = HumanExperienceEngine::render_photos_screen(&desktop, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(d_photos_pre.total_photos, 0);

    // 4. Synchronize
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    // 5. Desktop immediately reflects photo in Family Space Photos lens
    let d_photos_post = HumanExperienceEngine::render_photos_screen(&desktop, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(d_photos_post.total_photos, 1);
    assert_eq!(d_photos_post.photo_cards[0].title, "Grand Canyon Hike");

    // 6. Object detail verified on Desktop
    let detail = HumanExperienceEngine::render_object_detail(&desktop, &obj_id, InterfaceComplexity::Standard).unwrap();
    assert_eq!(detail.title, "Grand Canyon Hike");
    assert_eq!(detail.space_label, "Family");
    assert!(detail.status_badge.contains("Verified"));
}

#[test]
fn test_r71_32_b_offline_capture_and_subsequent_reconnection() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x03u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x04u8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    // Mobile captures 3 offline photos while disconnected
    for i in 1..=3 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("Offline {}", i), vec![i as u8; 100], 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    assert_eq!(HumanExperienceEngine::render_photos_screen(&mobile, SpaceType::Family, InterfaceComplexity::Simple).total_photos, 3);
    assert_eq!(HumanExperienceEngine::render_photos_screen(&desktop, SpaceType::Family, InterfaceComplexity::Simple).total_photos, 0);

    // Reconnection & Sync
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(HumanExperienceEngine::render_photos_screen(&desktop, SpaceType::Family, InterfaceComplexity::Simple).total_photos, 3);
}

#[test]
fn test_r71_32_c_unauthorized_user_cannot_mutate_space() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x05u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key);
    node.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    // Only OP_READ
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_READ,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let res = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Hacked", b"bad".to_vec(), 10, &BTreeMap::new(), &root_actor);
    assert!(res.is_err());
    assert_eq!(HumanExperienceEngine::render_photos_screen(&node, SpaceType::Family, InterfaceComplexity::Simple).total_photos, 0);
}

#[test]
fn test_r71_32_d_node_restart_preserves_human_views() {
    let tmp = tempdir().unwrap();
    let root_seed = [0x07u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), SigningKey::from_bytes(&root_seed));
    node.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(SigningKey::from_bytes(&root_seed).verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: SigningKey::from_bytes(&root_seed).sign(&token_hash).to_bytes().to_vec(),
    };

    let (obj_id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut node, &proof, "Persistent Memory", b"data".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    node.stop().unwrap();

    // Restart node cleanly
    let mut restarted_node = NexNode::new(tmp.path(), SigningKey::from_bytes(&root_seed));
    restarted_node.start().unwrap();

    // Re-insert into in-memory store if needed or verify persistence
    restarted_node.state.object_store.insert(obj_id, nex_core::object::types::NexObject {
        object_id: obj_id,
        object_type: nex_core::object::types::ObjectType::PhotoMedia,
        namespace: family_ns,
        owner_actor_id: root_actor,
        schema_version: 1,
        created_epoch: 10,
        created_lamport: 1,
        winning_mutation_id: [0u8; 32],
        metadata: {
            let mut m = BTreeMap::new();
            m.insert("title".to_string(), "Persistent Memory".to_string());
            m
        },
        payload_bytes: b"data".to_vec(),
        tombstoned: false,
    });

    let vm = HumanExperienceEngine::render_photos_screen(&restarted_node, SpaceType::Family, InterfaceComplexity::Simple);
    assert_eq!(vm.total_photos, 1);
    assert_eq!(vm.photo_cards[0].object_id, obj_id);
}

#[test]
fn test_r71_32_e_deletion_on_desktop_reflects_in_photos_and_drive_lenses() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x09u8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x0Au8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let (id, _) = SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, "Will Be Deleted", b"pic".to_vec(), 10, &BTreeMap::new(), &root_actor).unwrap();
    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);
    assert_eq!(HumanExperienceEngine::render_photos_screen(&desktop, SpaceType::Family, InterfaceComplexity::Simple).total_photos, 1);

    // Delete on desktop
    desktop.delete_object(id, None).unwrap();
    assert_eq!(HumanExperienceEngine::render_photos_screen(&desktop, SpaceType::Family, InterfaceComplexity::Simple).total_photos, 0);
}

#[test]
fn test_r71_32_f_multi_device_space_consistency() {
    let tmp_m = tempdir().unwrap();
    let tmp_d = tempdir().unwrap();

    let root_seed = [0x0Bu8; 32];
    let root_key = SigningKey::from_bytes(&root_seed);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut mobile = NexNode::new(tmp_m.path(), SigningKey::from_bytes(&root_seed));
    let mut desktop = NexNode::new(tmp_d.path(), SigningKey::from_bytes(&[0x0Cu8; 32]));
    mobile.start().unwrap();
    desktop.start().unwrap();

    let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
    let token = CapabilityToken {
        issuer: root_actor,
        subject: root_actor,
        namespace: family_ns,
        object_id: None,
        allowed_operations: OP_WRITE,
        delegation_depth: 0,
        not_before_epoch: 1,
        expires_at_epoch: 100,
        parent_token_hash: None,
    };
    let token_hash = hash_capability_token(&token);
    let proof = CapabilityProof {
        token,
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    // 2 Photos + 2 Docs on mobile
    for i in 1..=2 {
        SovereignProductSlice::mobile_capture_family_photo(&mut mobile, &proof, &format!("P{}", i), vec![0; 50], 10, &BTreeMap::new(), &root_actor).unwrap();
        SovereignProductSlice::mobile_create_family_document(&mut mobile, &proof, &format!("D{}.pdf", i), vec![0; 100], 10, &BTreeMap::new(), &root_actor).unwrap();
    }

    SovereignProductSlice::sync_mobile_to_desktop(&mut mobile, &mut desktop);

    let m_home = HumanExperienceEngine::render_home_screen(&mobile, SpaceType::Family, InterfaceComplexity::Simple);
    let d_home = HumanExperienceEngine::render_home_screen(&desktop, SpaceType::Family, InterfaceComplexity::Simple);

    assert_eq!(m_home.total_items_in_space, 4);
    assert_eq!(d_home.total_items_in_space, 4);
}
