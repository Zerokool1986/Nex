use sha2::{Sha256, Digest};
use nex_core::apps::compute::*;

#[test]
fn test_r64_3_a_fuel_exhaustion_trap() {
    let bytecode = vec![0x00; 100]; // 100 NOPs = 1000 fuel needed
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x01u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 100, // Only 100 fuel allocated (10 ops)
        memory_limit_bytes: 1024,
    };

    let err = ComputeEngine::execute_kernel(&job, &bytecode, &[]).unwrap_err();
    assert_eq!(err, ComputeError::FuelExhausted);
}

#[test]
fn test_r64_3_b_memory_limit_exceeded_trap() {
    let bytecode = vec![0x01]; // Op 0x01: copy input to output
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x02u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 1000,
        memory_limit_bytes: 10, // Max 10 bytes output
    };

    let big_input = vec![0xEEu8; 100]; // 100 bytes input
    let err = ComputeEngine::execute_kernel(&job, &bytecode, &[big_input]).unwrap_err();
    assert_eq!(err, ComputeError::MemoryLimitExceeded);
}

#[test]
fn test_r64_3_c_bytecode_hash_mismatch() {
    let bytecode = vec![0x01];
    let job = ComputeJobDescriptor {
        job_id: [0x03u8; 32],
        wasm_bytecode_hash: [0xFFu8; 32], // Wrong hash
        input_object_ids: vec![],
        fuel_limit: 1000,
        memory_limit_bytes: 1024,
    };

    let err = ComputeEngine::execute_kernel(&job, &bytecode, &[]).unwrap_err();
    assert_eq!(err, ComputeError::InvalidBytecode);
}

#[test]
fn test_r64_3_d_empty_bytecode_rejected() {
    let job = ComputeJobDescriptor {
        job_id: [0x04u8; 32],
        wasm_bytecode_hash: [0u8; 32],
        input_object_ids: vec![],
        fuel_limit: 1000,
        memory_limit_bytes: 1024,
    };

    let err = ComputeEngine::execute_kernel(&job, &[], &[]).unwrap_err();
    assert_eq!(err, ComputeError::InvalidBytecode);
}

#[test]
fn test_r64_3_e_explicit_execution_trap() {
    let bytecode = vec![0xFF]; // Op 0xFF: Explicit Trap
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x05u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 1000,
        memory_limit_bytes: 1024,
    };

    let err = ComputeEngine::execute_kernel(&job, &bytecode, &[]).unwrap_err();
    assert_eq!(err, ComputeError::ExecutionTrap("Kernel raised manual trap".to_string()));
}

#[test]
fn test_r64_3_f_zero_regression_fuel_lifecycle() {
    let bytecode = vec![0x00; 10];
    let mut hasher = Sha256::new();
    hasher.update(&bytecode);
    let bytecode_hash: [u8; 32] = hasher.finalize().into();

    let job = ComputeJobDescriptor {
        job_id: [0x06u8; 32],
        wasm_bytecode_hash: bytecode_hash,
        input_object_ids: vec![],
        fuel_limit: 100,
        memory_limit_bytes: 1024,
    };

    let res = ComputeEngine::execute_kernel(&job, &bytecode, &[]).unwrap();
    assert_eq!(res.fuel_consumed, 100);
}
