use std::ffi::{CString, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;
use nex_core::ffi::c_abi::*;
use nex_core::ffi::handle::*;

extern "C" fn test_callback_counter(
    _payload_ptr: *const u8,
    _payload_len: usize,
    user_data: *mut c_void,
) {
    let counter = unsafe { &*(user_data as *const AtomicUsize) };
    counter.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn test_r56_2_a_concurrent_multi_threaded_c_abi_invocations() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [11u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    let handle_usize = handle as usize;
    let mut threads = vec![];

    for i in 0..8 {
        let t = thread::spawn(move || {
            let h = handle_usize as *mut NexHandle;
            let req = serde_json::json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "nex_ping",
                "params": {}
            }).to_string();

            let mut resp_ptr: *mut u8 = std::ptr::null_mut();
            let mut resp_len: usize = 0;

            let status = nex_runtime_invoke(
                h,
                req.as_ptr(),
                req.len(),
                &mut resp_ptr,
                &mut resp_len,
            );
            assert_eq!(status, NEX_SUCCESS);
            nex_buffer_free(resp_ptr, resp_len);
        });
        threads.push(t);
    }

    for t in threads {
        t.join().unwrap();
    }

    nex_runtime_free(handle);
}

#[test]
fn test_r56_2_b_topic_based_event_subscription_and_dispatch() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [22u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    let counter = Arc::new(AtomicUsize::new(0));
    let topic_c = CString::new("sync.completed").unwrap();

    let sub_id = nex_runtime_subscribe(
        handle,
        topic_c.as_ptr(),
        test_callback_counter,
        Arc::as_ptr(&counter) as *mut c_void,
    );
    assert!(sub_id > 0, "Subscription ID must be positive");

    let handle_id = unsafe { (*handle).handle_id };
    let instance = HandleRegistry::get(handle_id).unwrap();

    // Emit event for matching topic
    instance.emit_event("sync.completed", b"{\"status\":\"converged\"}");
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Emit event for non-matching topic
    instance.emit_event("chat.received", b"{\"msg\":\"hello\"}");
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Emit another matching event
    instance.emit_event("sync.completed", b"{\"status\":\"updated\"}");
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    nex_runtime_free(handle);
}

#[test]
fn test_r56_2_c_wildcard_event_subscription() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [33u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    let counter = Arc::new(AtomicUsize::new(0));
    let wildcard_topic = CString::new("*").unwrap();

    let sub_id = nex_runtime_subscribe(
        handle,
        wildcard_topic.as_ptr(),
        test_callback_counter,
        Arc::as_ptr(&counter) as *mut c_void,
    );
    assert!(sub_id > 0);

    let handle_id = unsafe { (*handle).handle_id };
    let instance = HandleRegistry::get(handle_id).unwrap();

    instance.emit_event("drive.created", b"{}");
    instance.emit_event("chat.message", b"{}");
    instance.emit_event("vault.unlocked", b"{}");

    assert_eq!(counter.load(Ordering::SeqCst), 3);
    nex_runtime_free(handle);
}

#[test]
fn test_r56_2_d_unsubscribe_semantics() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [44u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    let counter = Arc::new(AtomicUsize::new(0));
    let topic_c = CString::new("test.event").unwrap();

    let sub_id = nex_runtime_subscribe(
        handle,
        topic_c.as_ptr(),
        test_callback_counter,
        Arc::as_ptr(&counter) as *mut c_void,
    );

    let handle_id = unsafe { (*handle).handle_id };
    let instance = HandleRegistry::get(handle_id).unwrap();

    instance.emit_event("test.event", b"1");
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Unsubscribe
    assert_eq!(nex_runtime_unsubscribe(handle, sub_id as u64), NEX_SUCCESS);

    // Emit again; counter must not increment
    instance.emit_event("test.event", b"2");
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Unsubscribe again returns not found
    assert_eq!(nex_runtime_unsubscribe(handle, sub_id as u64), NEX_ERR_OBJECT_NOT_FOUND);

    nex_runtime_free(handle);
}

#[test]
fn test_r56_2_e_graceful_shutdown_event_dispatcher_drain() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [55u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    let counter = Arc::new(AtomicUsize::new(0));
    let topic_c = CString::new("drain.test").unwrap();

    nex_runtime_subscribe(
        handle,
        topic_c.as_ptr(),
        test_callback_counter,
        Arc::as_ptr(&counter) as *mut c_void,
    );

    // Stop runtime
    assert_eq!(nex_runtime_stop(handle), NEX_SUCCESS);

    let handle_id = unsafe { (*handle).handle_id };
    let instance = HandleRegistry::get(handle_id).unwrap();

    // Emitting after shutdown must be ignored
    instance.emit_event("drain.test", b"dropped");
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    nex_runtime_free(handle);
}

#[test]
fn test_r56_2_f_zero_regression_under_concurrent_load() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [66u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    let counter = Arc::new(AtomicUsize::new(0));
    let topic_c = CString::new("stress.event").unwrap();

    nex_runtime_subscribe(
        handle,
        topic_c.as_ptr(),
        test_callback_counter,
        Arc::as_ptr(&counter) as *mut c_void,
    );

    let handle_id = unsafe { (*handle).handle_id };
    let instance = HandleRegistry::get(handle_id).unwrap();

    for i in 0..100 {
        instance.emit_event("stress.event", format!("payload_{}", i).as_bytes());
    }

    assert_eq!(counter.load(Ordering::SeqCst), 100);
    nex_runtime_free(handle);
}
