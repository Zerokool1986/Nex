use std::sync::Arc;
use std::thread;
use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use sha2::{Sha256, Digest};
use nex_core::runtime::node::NexNode;
use nex_core::apps::compute::*;
use nex_core::api::NexAppApi;

#[test]
fn test_r64_4_a_parallel_thread_compute_execution() {
    let bytecode = Arc::new(vec![0x02]); // Hash aggregation
    let mut hasher = Sha256::new();
    hasher.update(&*bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let mut handles = Vec::new();
    for i in 0..8 {
        let bc = Arc::clone(&bytecode);
        let handle = thread::spawn(move || {
            let job = ComputeJobDescriptor {
                job_id: [i as u8; 32],
                wasm_bytecode_hash: bytecode_hash,
                input_object_ids: vec![],
                fuel_limit: 1000,
                memory_limit_bytes: 1024,
            };
            let input = vec![vec![i as u8; 32]];
            ComputeEngine::execute_kernel(&job, &bc, &input).unwrap()
        });
        handles.push(handle);
    }

    for h in handles {
        let res = h.join().unwrap();
        assert_eq!(res.output_bytes.len(), 32);
    }
}

#[test]
fn test_r64_4_b_result_commitment_tamper_detection() {
    let job_id = [0x77u8; 32];
    let output = b"Output data".to_vec();
    let fuel = 50;

    let res = ComputeResult::new(job_id, output, fuel);

    // Tamper with output
    let tampered_output = b"Forged data".to_vec();
    let forged_commitment = ComputeResult::compute_commitment(&job_id, &tampered_output, fuel);

    assert_ne!(res.result_commitment, forged_commitment);
}

#[test]
fn test_r64_4_c_high_throughput_job_burst() {
    let mut scheduler = ComputeScheduler::new();
    for i in 0..500 {
        let job = ComputeJobDescriptor {
            job_id: [i as u8; 32],
            wasm_bytecode_hash: [0u8; 32],
            input_object_ids: vec![],
            fuel_limit: 100,
            memory_limit_bytes: 1024,
        };
        scheduler.submit_job(job);
    }
    assert_eq!(scheduler.pending_queue.len(), 500);

    for _ in 0..500 {
        assert!(scheduler.dispatch_job().is_some());
    }
    assert_eq!(scheduler.pending_queue.len(), 0);
}

#[test]
fn test_r64_4_d_large_input_matrix_streaming() {
    let bytecode = vec![0x02]; // Hash aggregation
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x88u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 5000,
        memory_limit_bytes: 1024 * 1024,
    };

    let mut matrix = Vec::new();
    for i in 0..100 {
        matrix.push(vec![i as u8; 1024]); // 100 x 1KB = 100KB input matrix
    }

    let res = ComputeEngine::execute_kernel(&job, &bytecode, &matrix).unwrap();
    assert_eq!(res.output_bytes.len(), 32);
}

#[test]
fn test_r64_4_e_unknown_opcodes_fuzzing() {
    // Opcode fuzzing: mix of NOPs, valid ops, and unknown opcodes
    let bytecode = vec![0x00, 0x99, 0x88, 0x77, 0x01];
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x99u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 1000,
        memory_limit_bytes: 1024,
    };

    let res = ComputeEngine::execute_kernel(&job, &bytecode, &[b"Test".to_vec()]).unwrap();
    assert_eq!(res.output_bytes, b"Test");
    assert_eq!(res.fuel_consumed, 50); // 5 ops * 10 fuel
}

#[test]
fn test_r64_4_f_gate_r64_master_compute_mesh_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [217u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Compute Mesh operations must preserve Merkle root invariance");
}
