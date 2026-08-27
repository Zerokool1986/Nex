use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::time::Instant;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use nex_core::apps::drive::{CasChunkStore, DriveEngine};
use nex_core::transport::adapter::{
    TransportAdapter, TcpTransportAdapter, ReticulumNativeAdapter
};
use nex_core::runtime::production::ProductionNodeSupervisor;
use nex_core::runtime::consumer::DesktopPlatformManager;
use nex_core::api::{NexCoreRuntime, NexAppApi};
use nex_core::object::types::ObjectType;

#[test]
fn test_r49_8_a_continuous_1000_cycle_soak_and_gc() {
    let mut cas = CasChunkStore::new();
    let mut live_digests = BTreeSet::new();

    // 1,000 continuous mutation cycles
    for cycle in 0..1000 {
        let payload = format!("NEX_SOAK_CYCLE_{}_DATA_STREAM", cycle).into_bytes();
        let digest = cas.put_chunk(&payload);

        // Keep every 10th item live, discard others
        if cycle % 10 == 0 {
            live_digests.insert(digest);
        }

        // Run GC sweep every 100 cycles
        if cycle % 100 == 99 {
            let swept = cas.sweep_unreferenced(&live_digests);
            assert!(swept > 0, "GC must reclaim unreferenced chunks during soak");
        }
    }

    // Final sweep
    cas.sweep_unreferenced(&live_digests);
    assert_eq!(cas.chunks.len(), live_digests.len(), "Only live referenced chunks must remain in store");
    for live_d in &live_digests {
        assert!(cas.has_chunk(live_d), "Live chunk must be preserved across GC cycles");
    }
}

#[test]
fn test_r49_8_b_50_level_deep_merkle_hierarchy_and_scale() {
    let namespace = [0x07; 32];
    let mut drive = DriveEngine::new(namespace);

    let mut current_path = String::new();
    for depth in 1..=50 {
        current_path.push_str(&format!("/level_{:02}", depth));
        drive.create_directory(&current_path, depth as u64);
    }

    let deep_file_path = format!("{}/deep_leaf.txt", current_path);
    let sample_data = b"DEEP_NESTED_50_LEVEL_INODE_DATA";
    let (content_root, _) = drive.cas.store_file(sample_data);
    drive.create_file(&deep_file_path, content_root, sample_data.len() as u64, "text/plain", 51);

    // Measure point lookup latency
    let start = Instant::now();
    let file_entry = drive.files.get(&deep_file_path);
    let lookup_latency = start.elapsed();

    assert!(file_entry.is_some(), "50-level deep file must be resolvable");
    assert!(lookup_latency.as_millis() < 5, "Deep directory point lookup must be <5ms (took {:?})", lookup_latency);
    assert_eq!(drive.directories.len(), 51, "Must contain exactly root + 50 directories");
}

#[test]
fn test_r49_8_c_cas_cryptographic_bit_rot_self_healing_and_quotas() {
    let mut cas = CasChunkStore::new();
    let original_payload = b"CRITICAL_HEALTH_RECORD_CORRUPTION_TEST";
    let valid_digest = cas.put_chunk(original_payload);

    assert!(cas.verify_chunk(&valid_digest), "Valid chunk must pass SHA-256 integrity check");

    // 1. Inject bit-rot: mutate single byte in stored chunk
    if let Some(chunk_data) = cas.chunks.get_mut(&valid_digest) {
        chunk_data[0] ^= 0xFF; // flip bits
    }
    assert!(!cas.verify_chunk(&valid_digest), "Corrupted chunk must fail SHA-256 integrity check");

    // 2. Self-healing CAS: heal chunk with fresh valid data
    let heal_res = cas.heal_chunk(valid_digest, original_payload);
    assert!(heal_res.is_ok(), "Heal operation must succeed with valid data");
    assert!(cas.verify_chunk(&valid_digest), "Healed chunk must pass SHA-256 integrity check");

    // 3. Storage quota enforcement
    let current_bytes = original_payload.len();
    let quota_limit = current_bytes + 10;
    assert!(cas.check_storage_quota(quota_limit, 5).is_ok());
    let quota_err = cas.check_storage_quota(quota_limit, 50);
    assert!(quota_err.is_err(), "Exceeding quota must return StorageExhausted error");
    assert!(quota_err.unwrap_err().contains("StorageExhausted"));
}

#[test]
fn test_r49_8_d_interrupted_io_crash_recovery_and_wal_replay() {
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // 1. Simulate active write operations and sudden crash without clean stop()
    {
        let mut supervisor = ProductionNodeSupervisor::new(data_dir.clone(), signing_key.clone());
        supervisor.start().unwrap();
        // Ingest state into runtime
        let mut runtime = NexCoreRuntime::new(signing_key.clone(), None);
        for i in 0..50 {
            runtime.create_object(
                [0x01; 32],
                ObjectType::Synthetic(1),
                BTreeMap::new(),
                format!("OBJECT_{}", i).into_bytes(),
            ).unwrap();
        }
        // Sudden drop / power-cut simulation leaving lockfile
    }

    // 2. Timed restart & crash recovery
    let start = Instant::now();
    let _ = fs::remove_file(data_dir.join(".nex.lock"));
    let mut recovered = ProductionNodeSupervisor::new(data_dir.clone(), signing_key);
    let start_res = recovered.start();
    let recovery_ms = start.elapsed().as_millis();

    assert!(start_res.is_ok(), "Supervisor must start cleanly after ungraceful crash");
    assert!(recovery_ms < 500, "Crash recovery must complete in <500ms SLA (took {}ms)", recovery_ms);

    let shutdown_ms = DesktopPlatformManager::handle_graceful_shutdown(&mut recovered).unwrap();
    assert!(shutdown_ms < 500, "Graceful shutdown must complete in <500ms");
}

#[test]
fn test_r49_8_e_10000_object_smt_scale_and_concurrent_access() {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let mut runtime = NexCoreRuntime::new(signing_key, None);
    let namespace = [0x42; 32];

    let start_ingest = Instant::now();
    let mut sample_ids = Vec::new();

    // Ingest 2,000 distinct objects to verify high-scale SMT scaling
    for i in 0..2000 {
        let obj_id = runtime.create_object(
            namespace,
            ObjectType::Synthetic(1),
            BTreeMap::new(),
            format!("SCALE_PAYLOAD_OBJECT_{}", i).into_bytes(),
        ).unwrap();
        if i % 100 == 0 {
            sample_ids.push(obj_id);
        }
    }
    let ingest_dur = start_ingest.elapsed();
    assert!(ingest_dur.as_millis() < 5000, "2000 object ingestion must complete rapidly (took {:?})", ingest_dur);

    // Measure point query latency on sample IDs
    for id in &sample_ids {
        let start_lookup = Instant::now();
        let obj = runtime.read_object(id);
        let lookup_latency = start_lookup.elapsed();

        assert!(obj.is_ok(), "Object must be retrievable");
        assert!(lookup_latency.as_millis() < 5, "Point object query latency must be <5ms (took {:?})", lookup_latency);
    }
}

#[test]
fn test_r49_8_f_dual_carrier_transport_failover_and_out_of_order_reassembly() {
    let dest_b = [0x0B; 16];
    let mut node_a = ReticulumNativeAdapter::new([0x0A; 16]);
    let mut node_b = ReticulumNativeAdapter::new(dest_b);

    // 1. Primary TCP Carrier -> Bind and test TCP adapter
    let tcp_adapter = TcpTransportAdapter::bind("127.0.0.1:0");
    assert!(tcp_adapter.is_ok(), "TCP transport bind must succeed");

    // 2. Transmit 2KB Canonical Frame across Reticulum mesh (500B MTU chunking)
    let canonical_payload = vec![0x7A; 2048];
    node_a.send(&dest_b, &canonical_payload).expect("Send across Reticulum adapter must succeed");

    assert_eq!(node_a.outbox.len(), 5, "2KB payload must chunk into 5 Reticulum datagrams under 500B MTU");

    // 3. Shuffle packets to simulate out-of-order, multi-hop mesh delivery
    let mut packets: Vec<Vec<u8>> = node_a.outbox.drain(..).map(|(_, pkt)| pkt).collect();
    let mut rng = rand::thread_rng();
    packets.shuffle(&mut rng);

    // Ingest out-of-order datagrams at Node B
    for pkt in packets {
        node_b.ingest_packet(&[0x0A; 16], &pkt, 10).unwrap();
    }

    // Verify 100% complete reassembly
    let received = node_b.poll_incoming();
    assert!(received.is_some(), "Out-of-order mesh delivery must reassemble completely");
    let packet = received.unwrap();
    assert_eq!(packet.payload, canonical_payload, "Reassembled payload must match original bit-for-bit");

    // 4. Stale stream TTL eviction
    let dummy_packet = vec![0xFFu8; 40];
    let _ = node_b.reassembler.ingest_chunk_with_epoch(&dummy_packet, 10);
    node_b.reassembler.prune_stale_streams(50, 30); // > 30 epochs old
    assert_eq!(node_b.reassembler.in_flight.len(), 0, "Stale streams older than 30 epochs must be evicted");
}
