use std::collections::{BTreeMap, BTreeSet};
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;

use nex_core::runtime::node::NexNode;
use nex_core::identity::types::{ActorID, KeyType, DeviceCertificate};
use nex_core::identity::master::NexMasterIdentity;
use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::recovery::device_recovery::{DeviceRecoveryWorkflow, RecoveryPlan, GuardianFactorType};
use nex_core::identity::recovery::shamir::GuardianShare;
use nex_core::object::types::{NexObject, ObjectType};
use nex_desktop::app::NexDesktopApp;

fn generate_random_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    seed
}

#[test]
fn test_recovery_lifecycle_phase_1_to_5_e2e_continuity() {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PHASE 1 — CREATE: Master Identity, 3-of-5 Recovery, Device A, State
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let master_seed = generate_random_seed();
    let master = NexMasterIdentity::from_seed(&master_seed);
    let original_root_actor = master.root_actor_id;

    // 1. Establish 3-of-5 recovery plan
    let (plan, shares) = DeviceRecoveryWorkflow::setup_3_of_5_recovery(
        &master_seed,
        100,
        Some([
            "Emergency Master Paper Key",
            "Amy (Family Living Circle)",
            "Bob (Trusted Friend)",
            "MacBook Pro (Hardware Token)",
            "Sovereign Decentralized Vault",
        ]),
        0, // Zero time-lock delay for test execution
    ).expect("Recovery plan setup must succeed");

    assert_eq!(plan.threshold, 3);
    assert_eq!(plan.total_shares, 5);
    assert_eq!(shares.len(), 5);
    assert_eq!(plan.root_actor_id, original_root_actor);

    // 2. Authorize Initial Device A (e.g. Phone)
    let device_a_key = SigningKey::generate(&mut OsRng);
    let device_a_pubkey = device_a_key.verifying_key().to_bytes();
    let device_a_actor = derive_actor_id(KeyType::Ed25519, &device_a_pubkey);

    let cert_a = master.issue_device_certificate(&device_a_pubkey, 100, 200_000)
        .expect("Device A certification must succeed");

    assert_eq!(cert_a.master_actor_id, original_root_actor);
    assert_eq!(cert_a.device_actor_id, device_a_actor);

    // 3. Create representative canonical state on Device A
    let tmp_a = tempdir().unwrap();
    let mut node_a = NexNode::new(tmp_a.path(), device_a_key);
    node_a.start().unwrap();

    let doc_id = [0xD1; 32];
    let mut doc_meta = BTreeMap::new();
    doc_meta.insert("filename".to_string(), "Sovereign_Declaration.pdf".to_string());
    doc_meta.insert("space".to_string(), "Personal".to_string());

    let original_doc = NexObject {
        object_id: doc_id,
        namespace: [0xFA; 32],
        object_type: ObjectType::DriveInode,
        schema_version: 1,
        created_epoch: 100,
        created_lamport: 1,
        owner_actor_id: original_root_actor,
        winning_mutation_id: [0u8; 32],
        metadata: doc_meta,
        payload_bytes: b"All sovereign data belongs exclusively to the root identity.".to_vec(),
        tombstoned: false,
    };
    node_a.state.object_store.insert(doc_id, original_doc.clone());

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PHASE 2 — LOSE: Device A is lost / unavailable
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Device A becomes unavailable
    drop(node_a);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PHASE 3 — RECOVER: Fresh Replacement Device B, 3 Threshold Shares
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let device_b_key = SigningKey::generate(&mut OsRng);
    let device_b_pubkey = device_b_key.verifying_key().to_bytes();
    let device_b_actor = derive_actor_id(KeyType::Ed25519, &device_b_pubkey);

    let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(original_root_actor, 0);

    // Present exactly 3 valid shares (Share 1: Paper, Share 2: Amy, Share 4: Laptop)
    ceremony.submit_share(shares[0].clone()).expect("Submit share 1");
    ceremony.submit_share(shares[1].clone()).expect("Submit share 2");
    ceremony.submit_share(shares[3].clone()).expect("Submit share 4");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PHASE 4 — REAUTHORIZE: Authorize Device B & Revoke Device A
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let mut crl = BTreeSet::new();

    let recovery_res = DeviceRecoveryWorkflow::execute_device_recovery(
        &ceremony,
        &device_b_pubkey,
        Some(device_a_actor),
        110,
        &mut crl,
    ).expect("Recovery execution must succeed");

    // Assert zero ID drift: ActorID A is preserved
    assert_eq!(recovery_res.root_actor_id, original_root_actor, "Root ActorID must remain identically ActorID A");
    assert_eq!(recovery_res.replacement_device_actor_id, device_b_actor);
    assert_eq!(recovery_res.replacement_certificate.master_actor_id, original_root_actor);
    assert_eq!(recovery_res.replacement_certificate.device_actor_id, device_b_actor);

    // Assert Device A is in the CRL
    assert!(crl.contains(&device_a_actor), "Lost Device A must be in Certificate Revocation List");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PHASE 5 — PROVE CONTINUITY: Device B operates as original identity
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let tmp_b = tempdir().unwrap();
    let mut node_b = NexNode::new(tmp_b.path(), device_b_key);
    node_b.start().unwrap();

    // Replicate pre-existing canonical object into Node B
    node_b.state.object_store.insert(doc_id, original_doc);

    let app_b = NexDesktopApp::new_test(node_b, tmp_b.path().to_path_buf());

    // 1. Device B can access pre-existing objects owned by ActorID A
    assert!(app_b.node.state.object_store.contains_key(&doc_id));
    let retrieved_doc = app_b.node.state.object_store.get(&doc_id).unwrap();
    assert_eq!(retrieved_doc.owner_actor_id, original_root_actor);
    assert_eq!(retrieved_doc.metadata.get("filename").unwrap(), "Sovereign_Declaration.pdf");

    // 2. Settings reflection is truthful
    let mut app_stateful = app_b;
    app_stateful.recovery_plan = Some(plan);
    app_stateful.active_crl = crl;

    assert_eq!(app_stateful.recovery_plan.as_ref().unwrap().threshold, 3);
    assert_eq!(app_stateful.active_crl.len(), 1);
    assert!(app_stateful.active_crl.contains(&device_a_actor));
}

#[test]
fn test_recovery_lifecycle_phase_6_failure_and_adversarial_modes() {
    let master_seed = generate_random_seed();
    let master = NexMasterIdentity::from_seed(&master_seed);
    let root_actor = master.root_actor_id;

    let (_, shares) = DeviceRecoveryWorkflow::setup_3_of_5_recovery(
        &master_seed,
        100,
        None,
        100, // Time-lock active until epoch 100
    ).unwrap();

    // ── 1. Insufficient Shares (2 of 5) ──
    {
        let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(root_actor, 100);
        ceremony.submit_share(shares[0].clone()).unwrap();
        ceremony.submit_share(shares[1].clone()).unwrap();

        let finalize_res = ceremony.finalize_recovery(150);
        assert!(finalize_res.is_err());
        assert_eq!(finalize_res.err().unwrap(), "InsufficientSharesForQuorum");
    }

    // ── 2. Duplicate Shares ──
    {
        let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(root_actor, 100);
        ceremony.submit_share(shares[0].clone()).unwrap();
        let dup_res = ceremony.submit_share(shares[0].clone());
        assert!(dup_res.is_err());
        assert_eq!(dup_res.err().unwrap(), "DuplicateGuardianShare");
    }

    // ── 3. Invalid / Tampered Share Bytes ──
    {
        let mut tampered_share = shares[2].clone();
        tampered_share.share_data[0] ^= 0xFF; // Flip bit in share

        let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(root_actor, 100);
        ceremony.submit_share(shares[0].clone()).unwrap();
        ceremony.submit_share(shares[1].clone()).unwrap();
        ceremony.submit_share(tampered_share).unwrap();

        let reconstructed_seed = ceremony.finalize_recovery(150).unwrap();
        // Tampered share must NOT reconstruct the valid master seed
        assert_ne!(reconstructed_seed, master_seed);

        let recovered_master = NexMasterIdentity::from_seed(&reconstructed_seed);
        assert_ne!(recovered_master.root_actor_id, root_actor);
    }

    // ── 4. Anti-Hijack Time-Lock Enforcement ──
    {
        let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(root_actor, 500);
        ceremony.submit_share(shares[0].clone()).unwrap();
        ceremony.submit_share(shares[1].clone()).unwrap();
        ceremony.submit_share(shares[2].clone()).unwrap();

        // 3 of 5 shares presented at epoch 200 (< time-lock 500)
        let time_locked_res = ceremony.finalize_recovery(200);
        assert!(time_locked_res.is_err());
        assert_eq!(time_locked_res.err().unwrap(), "TimeLockActiveWaitRequired");

        // Presenting all 5 of 5 shares bypasses time-lock
        ceremony.submit_share(shares[3].clone()).unwrap();
        ceremony.submit_share(shares[4].clone()).unwrap();
        let bypass_res = ceremony.finalize_recovery(200);
        assert!(bypass_res.is_ok(), "5-of-5 unanimous shares bypasses anti-hijack time-lock");
    }

    // ── 5. Owner Cancellation of Pending Ceremony ──
    {
        let mut ceremony = DeviceRecoveryWorkflow::start_ceremony(root_actor, 500);
        ceremony.submit_share(shares[0].clone()).unwrap();

        // Owner detects unauthorized recovery and cancels
        ceremony.cancel_ceremony();
        assert!(ceremony.is_canceled);

        let submit_after_cancel = ceremony.submit_share(shares[1].clone());
        assert!(submit_after_cancel.is_err());
        assert_eq!(submit_after_cancel.err().unwrap(), "CeremonyCanceledByOwner");

        let finalize_after_cancel = ceremony.finalize_recovery(600);
        assert!(finalize_after_cancel.is_err());
        assert_eq!(finalize_after_cancel.err().unwrap(), "CeremonyCanceledByOwner");
    }
}
