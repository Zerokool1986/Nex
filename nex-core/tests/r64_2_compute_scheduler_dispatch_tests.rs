use nex_core::apps::compute::*;

#[test]
fn test_r64_2_a_job_submission_and_dispatch_order() {
    let mut scheduler = ComputeScheduler::new();

    let job1 = ComputeJobDescriptor {
        job_id: [0x01u8; 32],
        wasm_bytecode_hash: [0u8; 32],
        input_object_ids: vec![],
        fuel_limit: 100,
        memory_limit_bytes: 1024,
    };
    let job2 = ComputeJobDescriptor {
        job_id: [0x02u8; 32],
        wasm_bytecode_hash: [0u8; 32],
        input_object_ids: vec![],
        fuel_limit: 200,
        memory_limit_bytes: 2048,
    };

    scheduler.submit_job(job1.clone());
    scheduler.submit_job(job2.clone());

    assert_eq!(scheduler.dispatch_job(), Some(job1));
    assert_eq!(scheduler.dispatch_job(), Some(job2));
    assert_eq!(scheduler.dispatch_job(), None);
}

#[test]
fn test_r64_2_b_record_and_retrieve_result() {
    let mut scheduler = ComputeScheduler::new();
    let job_id = [0xAAu8; 32];
    let worker_id = [0xBBu8; 32];
    let result = ComputeResult::new(job_id, vec![1, 2, 3, 4], 50);

    scheduler.record_result(job_id, worker_id, result.clone());

    let retrieved = scheduler.get_result(&job_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), &result);
}

#[test]
fn test_r64_2_c_empty_queue_dispatch() {
    let mut scheduler = ComputeScheduler::new();
    assert_eq!(scheduler.dispatch_job(), None);
}

#[test]
fn test_r64_2_d_unknown_job_result_returns_none() {
    let scheduler = ComputeScheduler::new();
    let unknown_job = [0x99u8; 32];
    assert_eq!(scheduler.get_result(&unknown_job), None);
}

#[test]
fn test_r64_2_e_multi_worker_results() {
    let mut scheduler = ComputeScheduler::new();
    for i in 0..10 {
        let job_id = [i as u8; 32];
        let worker_id = [i as u8 + 10; 32];
        let result = ComputeResult::new(job_id, vec![i as u8], (i as u64) * 10);
        scheduler.record_result(job_id, worker_id, result);
    }
    assert_eq!(scheduler.completed_jobs.len(), 10);
}

#[test]
fn test_r64_2_f_zero_regression_scheduler_lifecycle() {
    let mut scheduler = ComputeScheduler::new();
    for i in 0..5 {
        let job = ComputeJobDescriptor {
            job_id: [i as u8; 32],
            wasm_bytecode_hash: [0u8; 32],
            input_object_ids: vec![],
            fuel_limit: 100,
            memory_limit_bytes: 512,
        };
        scheduler.submit_job(job);
    }
    for _ in 0..5 {
        assert!(scheduler.dispatch_job().is_some());
    }
    assert!(scheduler.dispatch_job().is_none());
}
