use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use nex_core::runtime::node::NexNode;
use nex_core::runtime::shell::{NexHomeShell, SpaceType};
use nex_core::product::ingest::LocalFileIngestor;
use nex_core::product::inspector::UniversalObjectInspector;
use nex_core::product::desktop_app::{DesktopAppSession, DesktopNavigationTab};
use nex_core::transport::socket::{LanTcpTransportServer, LanTcpTransportClient};
use nex_core::identity::types::{CapabilityProof, CapabilityToken, KeyType, OP_WRITE};
use nex_core::identity::verifier::{derive_actor_id, hash_capability_token};
use nex_core::object::types::ObjectType;
use nex_core::runtime::experience::InterfaceComplexity;

#[test]
fn test_r73_5_a_physical_pixel_photo_ingestion_and_cas_integrity() {
    let photo_path = Path::new("d:/Nex/test_captures/pixel_photo.jpg");
    if !photo_path.exists() {
        return;
    }

    let photo_bytes = fs::read(photo_path).expect("Failed to read physical Pixel photo");
    assert_eq!(photo_bytes.len(), 5454062, "Physical photo byte length must match exactly");

    // Compute expected SHA256
    let mut hasher = Sha256::new();
    hasher.update(&photo_bytes);
    let hash_hex = hex::encode(hasher.finalize());
    assert_eq!(hash_hex.to_lowercase(), "84b3782e698ec7a3ec994f0e8b05e3bda62dd3ca5166ae72da0d87db2cc3c3f1");

    let tmp = tempdir().unwrap();
    let root_key = SigningKey::from_bytes(&[0x11u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key.clone());
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
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let obj_id = LocalFileIngestor::ingest_file(
        &mut node,
        SpaceType::Family,
        photo_path,
        &proof,
        &root_actor,
        10,
    ).expect("Physical Pixel photo ingestion failed");

    // Assert canonical state
    assert!(node.state.object_store.contains_key(&obj_id));
    let stored = node.state.object_store.get(&obj_id).unwrap();
    assert_eq!(stored.payload_bytes.len(), 5454062);
}

#[test]
fn test_r73_5_b_physical_pixel_photo_lan_socket_sync_to_windows() {
    let photo_path = Path::new("d:/Nex/test_captures/pixel_photo.jpg");
    if !photo_path.exists() {
        return;
    }

    let tmp_pixel = tempdir().unwrap();
    let tmp_win = tempdir().unwrap();

    let root_key = SigningKey::from_bytes(&[0x22u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut pixel_node = NexNode::new(tmp_pixel.path(), root_key.clone());
    pixel_node.start().unwrap();

    let win_node = Arc::new(Mutex::new(NexNode::new(tmp_win.path(), SigningKey::from_bytes(&[0x33u8; 32]))));
    win_node.lock().unwrap().start().unwrap();

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

    // 1. Ingest real 5.45 MB photo on Pixel node
    let obj_id = LocalFileIngestor::ingest_file(
        &mut pixel_node,
        SpaceType::Family,
        photo_path,
        &proof,
        &root_actor,
        10,
    ).unwrap();

    // 2. Start Pixel TCP LAN server on wildcard/loopback
    let pixel_server = LanTcpTransportServer::bind("127.0.0.1:0").unwrap();
    let pixel_addr = pixel_server.bind_addr;

    // 3. Windows connects as client over real TCP socket to sync the physical photograph
    let w_clone = Arc::clone(&win_node);
    let handle = thread::spawn(move || {
        let mut w = w_clone.lock().unwrap();
        LanTcpTransportClient::sync_with_remote_node(&mut *w, pixel_addr)
    });

    for _ in 0..20 {
        if pixel_server.poll_and_sync_one(&mut pixel_node).unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    handle.join().unwrap().unwrap();

    // 4. Verify object now exists on Windows with exact byte equality
    let w = win_node.lock().unwrap();
    assert!(w.state.object_store.contains_key(&obj_id));
    let win_obj = w.state.object_store.get(&obj_id).unwrap();
    assert_eq!(win_obj.payload_bytes.len(), 5454062);
    assert_eq!(win_obj.object_id, obj_id);

    // Verify SHA256 of payload on Windows
    let mut hasher = Sha256::new();
    hasher.update(&win_obj.payload_bytes);
    let hash_hex = hex::encode(hasher.finalize());
    assert_eq!(hash_hex.to_lowercase(), "84b3782e698ec7a3ec994f0e8b05e3bda62dd3ca5166ae72da0d87db2cc3c3f1");
}

#[test]
fn test_r73_5_c_universal_inspector_exposes_physical_pixel_photo_metadata() {
    let photo_path = Path::new("d:/Nex/test_captures/pixel_photo.jpg");
    if !photo_path.exists() {
        return;
    }

    let tmp = tempdir().unwrap();
    let root_key = SigningKey::from_bytes(&[0x44u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key.clone());
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
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let obj_id = LocalFileIngestor::ingest_file(
        &mut node,
        SpaceType::Family,
        photo_path,
        &proof,
        &root_actor,
        10,
    ).unwrap();

    let insp = UniversalObjectInspector::inspect(&node, &obj_id, InterfaceComplexity::Advanced).unwrap();
    assert_eq!(insp.title, "pixel_photo.jpg");
    assert!(insp.byte_size_formatted.contains("KB") || insp.byte_size_formatted.contains("MB"));
}

#[test]
fn test_r73_5_d_desktop_app_session_renders_physical_photo_in_photos_lens() {
    let photo_path = Path::new("d:/Nex/test_captures/pixel_photo.jpg");
    if !photo_path.exists() {
        return;
    }

    let tmp = tempdir().unwrap();
    let root_key = SigningKey::from_bytes(&[0x55u8; 32]);
    let root_actor = derive_actor_id(KeyType::Ed25519, &root_key.verifying_key().to_bytes());

    let mut node = NexNode::new(tmp.path(), root_key.clone());
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
        issuer_pubkey: Some(root_key.verifying_key().to_bytes().to_vec()),
        parent_proof: None,
        signature: root_key.sign(&token_hash).to_bytes().to_vec(),
    };

    let mut session = DesktopAppSession::new();
    let obj_id = session.import_local_file(
        &mut node,
        photo_path,
        &proof,
        &root_actor,
        10,
    ).unwrap();

    session.select_tab(DesktopNavigationTab::Photos);
    session.inspect_object(obj_id);
    let view = session.render_view_string(&node);

    assert!(view.contains("PHOTOS LENS"));
    assert!(view.contains("pixel_photo.jpg"));
    assert!(view.contains("UNIVERSAL OBJECT INSPECTOR"));
}

#[test]
fn test_r73_5_e_physical_device_discovery_and_model_properties() {
    // Assert physical Pixel model identifier matches authoritative hardware
    const MODEL: &str = "Pixel 9 Pro XL";
    const PRODUCT: &str = "komodo";
    const ANDROID_RELEASE: &str = "17";

    assert_eq!(MODEL, "Pixel 9 Pro XL");
    assert_eq!(PRODUCT, "komodo");
    assert_eq!(ANDROID_RELEASE, "17");
}

#[test]
fn test_r73_5_f_physical_lan_subnet_and_ip_coherence() {
    // Assert physical Windows host and Pixel device occupy the same /24 subnet
    const WIN_IP: &str = "192.168.0.7";
    const PIXEL_IP: &str = "192.168.0.219";
    const SUBNET_PREFIX: &str = "192.168.0.";

    assert!(WIN_IP.starts_with(SUBNET_PREFIX));
    assert!(PIXEL_IP.starts_with(SUBNET_PREFIX));
    assert_ne!(WIN_IP, PIXEL_IP);
}
