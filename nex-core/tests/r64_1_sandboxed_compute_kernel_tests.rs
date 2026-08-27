use sha2::{Sha256, Digest};
use nex_core::apps::compute::*;

#[test]
fn test_r64_1_a_identity_kernel_execution() {
    let bytecode = vec![0x01]; // Op 0x01: Identity transform
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x11u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![[0x01u8; 32]],
        fuel_limit: 1000,
        memory_limit_bytes: 1024 * 1024,
    };

    let inputs = vec![b"Hello, Sovereign Compute Mesh!".to_vec()];
    let res = ComputeEngine::execute_kernel(&job, &bytecode, &inputs).unwrap();

    assert_eq!(res.output_bytes, b"Hello, Sovereign Compute Mesh!");
    assert_eq!(res.fuel_consumed, 10);
}

#[test]
fn test_r64_1_b_hash_aggregation_kernel() {
    let bytecode = vec![0x02]; // Op 0x02: Hash aggregation
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x22u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![[0x01u8; 32], [0x02u8; 32]],
        fuel_limit: 1000,
        memory_limit_bytes: 1024 * 1024,
    };

    let inputs = vec![b"Input A".to_vec(), b"Input B".to_vec()];
    let res = ComputeEngine::execute_kernel(&job, &bytecode, &inputs).unwrap();

    let mut expected_hasher = Sha256::new();
    expected_hasher.update(b"Input A");
    expected_hasher.update(b"Input B");
    let expected: [u8; 32] = expected_hasher.finalize().into();

    assert_eq!(res.output_bytes, expected.to_vec());
}

#[test]
fn test_r64_1_c_bitwise_inversion_kernel() {
    let bytecode = vec![0x01, 0x03]; // Op 0x01 (copy), Op 0x03 (NOT)
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x33u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![[0x01u8; 32]],
        fuel_limit: 1000,
        memory_limit_bytes: 1024 * 1024,
    };

    let inputs = vec![vec![0x00, 0x0F, 0xF0, 0xFF]];
    let res = ComputeEngine::execute_kernel(&job, &bytecode, &inputs).unwrap();

    assert_eq!(res.output_bytes, vec![0xFF, 0xF0, 0x0F, 0x00]);
    assert_eq!(res.fuel_consumed, 20);
}

#[test]
fn test_r64_1_d_nop_fuel_consumption() {
    let bytecode = vec![0x00, 0x00, 0x00, 0x00, 0x00]; // 5 NOPs
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x44u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 1000,
        memory_limit_bytes: 1024 * 1024,
    };

    let res = ComputeEngine::execute_kernel(&job, &bytecode, &[]).unwrap();
    assert_eq!(res.fuel_consumed, 50);
    assert_eq!(res.output_bytes, Vec::<u8>::new());
}

#[test]
fn test_r64_1_e_deterministic_result_commitment() {
    let job_id = [0x55u8; 32];
    let output = b"Deterministic Result".to_vec();
    let fuel = 100;

    let res1 = ComputeResult::new(job_id, output.clone(), fuel);
    let res2 = ComputeResult::new(job_id, output, fuel);

    assert_eq!(res1.result_commitment, res2.result_commitment);
}

#[test]
fn test_r64_1_f_zero_regression_kernel_lifecycle() {
    let bytecode = vec![0x01];
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    for i in 0..10 {
        let job = ComputeJobDescriptor {
            job_id: [i as u8; 32],
            wasm_bytecode_hash: bytecode_hash,
            input_object_ids: vec![],
            fuel_limit: 500,
            memory_limit_bytes: 1024,
        };
        let res = ComputeEngine::execute_kernel(&job, &bytecode, &[vec![i as u8]]).unwrap();
        assert_eq!(res.output_bytes, vec![i as u8]);
    }
}
