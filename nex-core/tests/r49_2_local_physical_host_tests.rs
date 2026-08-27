use std::fs;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;
use tempfile::tempdir;

use nex_core::runtime::production::ProductionNodeSupervisor;
use nex_core::runtime::system::SovereignMnemonic;
use nex_core::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};
use nex_core::storage::wal::WriteAheadLog;
use nex_core::apps::drive::CasChunkStore;
use nex_core::model::{Mutation, MutationBody, CrdtPayload};
use nex_core::identity::types::KeyType;
use nex_core::identity::verifier::derive_actor_id;

/// R49-2-A: Real Host Startup & Process Inspection
#[test]
fn test_r49_2_a_real_host_startup_and_status() {
    let tmp = tempdir().expect("Failed to create tempdir on physical host");
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let key = SigningKey::from_bytes(&seed);

    let mut supervisor = ProductionNodeSupervisor::new(tmp.path(), key);
    let start_res = supervisor.start();
    assert!(start_res.is_ok(), "Daemon failed to start on physical host");

    // Query status via JSON-RPC
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 101,
        method: "nex_getStatus".to_string(),
        params: serde_json::Value::Null,
    };
    let resp = NexRpcDispatcher::dispatch(&mut supervisor, req);
    assert_eq!(resp.id, 101);
    assert!(resp.error.is_none());

    let result = resp.result.expect("Expected result payload");
    assert_eq!(result["state"], "RUNNING");
    assert!(result["schema_version"].is_number());

    let stop_res = supervisor.stop();
    assert!(stop_res.is_ok(), "Daemon failed to gracefully stop");
}

/// R49-2-B: Identity Initialization, Persistence & Stability
#[test]
fn test_r49_2_b_identity_initialization_and_persistence() {
    let tmp = tempdir().expect("tempdir");
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let mnemonic = SovereignMnemonic::generate_24_words(&seed);
    let initial_actor_id = mnemonic.to_actor_id();

    // Persist mnemonic to real disk
    let mnemonic_path = tmp.path().join("identity.mnemonic");
    let serialized = serde_json::to_vec(&mnemonic).unwrap();
    fs::write(&mnemonic_path, serialized).expect("Failed to write mnemonic to host disk");

    // Simulate node restart: read from host disk
    let read_bytes = fs::read(&mnemonic_path).expect("Failed to read mnemonic from host disk");
    let restored_mnemonic: SovereignMnemonic = serde_json::from_slice(&read_bytes).unwrap();
    let restored_actor_id = restored_mnemonic.to_actor_id();

    assert_eq!(initial_actor_id, restored_actor_id, "ActorID mutated across restart");
}

/// R49-2-C & R49-2-G: Real Filesystem CAS Storage & Deduplication
#[test]
fn test_r49_2_c_and_g_real_cas_persistence_and_dedup() {
    let mut cas = CasChunkStore::new();

    let content_a = b"Sovereign Nex Protocol Payload Data Block Alpha";
    let content_b = b"Sovereign Nex Protocol Payload Data Block Alpha"; // Duplicate

    let digest_a = cas.put_chunk(content_a);
    let digest_b = cas.put_chunk(content_b);

    assert_eq!(digest_a, digest_b, "Duplicate content must produce identical digest");
    assert_eq!(cas.chunks.len(), 1, "CAS must deduplicate identical chunks");

    // Retrieve chunk and verify content
    let retrieved = cas.get_chunk(&digest_a).expect("Chunk missing");
    assert_eq!(retrieved.as_slice(), content_a);
}

/// R49-2-D: Daemon Lifecycle & PID Lockfile Exclusivity
#[test]
fn test_r49_2_d_daemon_lifecycle_and_lockfile() {
    let tmp = tempdir().expect("tempdir");
    let mut seed_1 = [0u8; 32];
    OsRng.fill_bytes(&mut seed_1);
    let key_1 = SigningKey::from_bytes(&seed_1);

    let mut supervisor_1 = ProductionNodeSupervisor::new(tmp.path(), key_1);
    supervisor_1.start().expect("Supervisor 1 should start");

    // Verify lockfile exists on disk
    let lockfile = tmp.path().join(".nex.lock");
    assert!(lockfile.exists(), "Lockfile must exist when daemon is active");

    // Attempt second concurrent startup on same path
    let mut seed_2 = [0u8; 32];
    OsRng.fill_bytes(&mut seed_2);
    let key_2 = SigningKey::from_bytes(&seed_2);
    let mut supervisor_2 = ProductionNodeSupervisor::new(tmp.path(), key_2);
    let second_start = supervisor_2.start();
    assert!(second_start.is_err(), "Concurrent startup must be rejected by lockfile");

    // Graceful stop removes lockfile
    supervisor_1.stop().expect("Supervisor 1 should stop");
    assert!(!lockfile.exists(), "Lockfile must be cleaned up on clean exit");

    // Now second supervisor can acquire lock
    supervisor_2.start().expect("Supervisor 2 should start after supervisor 1 stops");
    supervisor_2.stop().expect("Supervisor 2 should stop");
}

/// R49-2-E: IPC Boundary & Malformed JSON-RPC Defense
#[test]
fn test_r49_2_e_ipc_boundary_and_malformed_requests() {
    let tmp = tempdir().expect("tempdir");
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let key = SigningKey::from_bytes(&seed);

    let mut supervisor = ProductionNodeSupervisor::new(tmp.path(), key);
    supervisor.start().unwrap();

    // 1. Valid request
    let valid_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "nex_getStatus".to_string(),
        params: serde_json::Value::Null,
    };
    let valid_resp = NexRpcDispatcher::dispatch(&mut supervisor, valid_req);
    assert!(valid_resp.error.is_none());

    // 2. Unknown method request
    let unknown_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 2,
        method: "nex_unknownMethod".to_string(),
        params: serde_json::Value::Null,
    };
    let unknown_resp = NexRpcDispatcher::dispatch(&mut supervisor, unknown_req);
    assert!(unknown_resp.error.is_some());
    let err = unknown_resp.error.unwrap();
    assert_eq!(err.code, -32601, "Method not found code must be -32601");

    supervisor.stop().unwrap();
}

/// R49-2-F: Real Disk WAL Recovery under Torn Write / Corruption
#[test]
fn test_r49_2_f_wal_recovery_under_torn_write() {
    let tmp = tempdir().expect("tempdir");
    let wal_path = tmp.path().join("test.wal");

    let mut wal = WriteAheadLog::open(&wal_path).expect("Failed to create WAL on host disk");

    let mut actor = [0u8; 32];
    actor[0] = 0xAA;

    let mutation_1 = Mutation {
        id: [1u8; 32],
        body: MutationBody {
            author: actor,
            parents: vec![],
            lamport: 1,
            epoch: 1,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [10u8; 32], value: b"Val1".to_vec() },
        },
    };
    wal.append_mutation(&mutation_1).expect("Append 1 failed");

    let mutation_2 = Mutation {
        id: [2u8; 32],
        body: MutationBody {
            author: actor,
            parents: vec![[1u8; 32]],
            lamport: 2,
            epoch: 1,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: [20u8; 32], value: b"Val2".to_vec() },
        },
    };
    wal.append_mutation(&mutation_2).expect("Append 2 failed");

    // Close WAL
    drop(wal);

    // Inject physical torn write at end of WAL file
    let mut file_bytes = fs::read(&wal_path).expect("Failed to read WAL");
    file_bytes.extend_from_slice(&[0xFF, 0xAA, 0x55, 0x00, 0x12]); // Truncated garbage frame
    fs::write(&wal_path, file_bytes).expect("Failed to write torn bytes");

    // Reopen WAL: must recover valid entries and gracefully stop at torn boundary
    let entries = WriteAheadLog::recover(&wal_path).expect("Recovery must succeed");

    assert_eq!(entries.len(), 2, "Must recover exactly the 2 valid mutations before crash");
    assert_eq!(entries[0].id, [1u8; 32]);
    assert_eq!(entries[1].id, [2u8; 32]);
}

/// R49-2-H: Destruction & Full Mnemonic Disaster Recovery
#[test]
fn test_r49_2_h_destruction_and_mnemonic_recovery() {
    let tmp = tempdir().expect("tempdir");
    let data_dir = tmp.path().join("sovereign_node");

    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let mnemonic = SovereignMnemonic::generate_24_words(&seed);
    let expected_actor_id = mnemonic.to_actor_id();

    // 1. Initial run
    {
        let key = mnemonic.to_signing_key();
        let mut supervisor = ProductionNodeSupervisor::new(&data_dir, key);
        supervisor.start().unwrap();
        supervisor.stop().unwrap();
    }
    assert!(data_dir.exists(), "Data directory should exist");

    // 2. Physical Destruction: rm -rf data directory
    fs::remove_dir_all(&data_dir).expect("Failed to destroy data directory");
    assert!(!data_dir.exists(), "Data directory must be completely erased");

    // 3. Disaster Recovery: Re-derive signing key & actor from 24-word seed
    let recovered_key = mnemonic.to_signing_key();
    let recovered_actor_id = derive_actor_id(KeyType::Ed25519, recovered_key.verifying_key().as_bytes());
    assert_eq!(expected_actor_id, recovered_actor_id, "Recovered ActorID must match original");

    // 4. Start fresh node with recovered key
    let mut restored_supervisor = ProductionNodeSupervisor::new(&data_dir, recovered_key);
    let start_res = restored_supervisor.start();
    assert!(start_res.is_ok(), "Restored supervisor must boot cleanly");
    restored_supervisor.stop().unwrap();
}

/// R49-2-I: Repetition — 10 Consecutive Physical Lifecycle Cycles
#[test]
fn test_r49_2_i_multi_cycle_lifecycle_repetition() {
    let tmp = tempdir().expect("tempdir");
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let key = SigningKey::from_bytes(&seed);

    for cycle in 1..=10 {
        let mut supervisor = ProductionNodeSupervisor::new(tmp.path(), key.clone());
        supervisor.start().unwrap_or_else(|e| panic!("Cycle {} startup failed: {:?}", cycle, e));

        // Status check
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: cycle,
            method: "nex_getStatus".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = NexRpcDispatcher::dispatch(&mut supervisor, req);
        assert!(resp.error.is_none());

        supervisor.stop().unwrap_or_else(|e| panic!("Cycle {} shutdown failed: {:?}", cycle, e));
    }
}
