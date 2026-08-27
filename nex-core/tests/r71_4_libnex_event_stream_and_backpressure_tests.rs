use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use nex_core::ffi::handle::*;

#[test]
fn test_r71_4_a_event_subscription_and_dispatch() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let seed = [0x12u8; 32];
    let runtime = RuntimeInstance::new(1, temp_dir.path().to_path_buf(), seed).expect("Runtime creation failed");

    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received);

    let sub_id = runtime.subscribe("object/created", move |payload| {
        let mut list = received_clone.lock().unwrap();
        list.push(payload.to_vec());
    });
    assert!(sub_id > 0);

    // Emit event
    let event_data = b"object_id:12345";
    runtime.emit_event("object/created", event_data);

    let list = received.lock().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0], event_data);
}

#[test]
fn test_r71_4_b_topic_filtered_event_isolation() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let seed = [0x34u8; 32];
    let runtime = RuntimeInstance::new(2, temp_dir.path().to_path_buf(), seed).expect("Runtime creation failed");

    let drive_events = Arc::new(Mutex::new(Vec::new()));
    let drive_clone = Arc::clone(&drive_events);

    runtime.subscribe("drive/sync", move |payload| {
        drive_clone.lock().unwrap().push(payload.to_vec());
    });

    // Emit to unrelated topic
    runtime.emit_event("chat/message", b"hello chat");

    assert_eq!(drive_events.lock().unwrap().len(), 0, "Subscriber must not receive events for unrelated topics");

    // Emit to subscribed topic
    runtime.emit_event("drive/sync", b"sync complete");
    assert_eq!(drive_events.lock().unwrap().len(), 1);
}

#[test]
fn test_r71_4_c_unsubscribe_lifecycle_safety() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let seed = [0x56u8; 32];
    let runtime = RuntimeInstance::new(3, temp_dir.path().to_path_buf(), seed).expect("Runtime creation failed");

    let counter = Arc::new(Mutex::new(0));
    let counter_clone = Arc::clone(&counter);

    let sub_id = runtime.subscribe("telemetry", move |_| {
        let mut c = counter_clone.lock().unwrap();
        *c += 1;
    });

    runtime.emit_event("telemetry", b"ping 1");
    assert_eq!(*counter.lock().unwrap(), 1);

    // Unsubscribe
    assert!(runtime.unsubscribe(sub_id));

    runtime.emit_event("telemetry", b"ping 2");
    assert_eq!(*counter.lock().unwrap(), 1, "Unsubscribed callback must not be invoked");
}

#[test]
fn test_r71_4_d_multi_subscriber_fanout() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let seed = [0x78u8; 32];
    let runtime = RuntimeInstance::new(4, temp_dir.path().to_path_buf(), seed).expect("Runtime creation failed");

    let sub1_count = Arc::new(Mutex::new(0));
    let sub2_count = Arc::new(Mutex::new(0));

    let c1 = Arc::clone(&sub1_count);
    let c2 = Arc::clone(&sub2_count);

    runtime.subscribe("broadcast", move |_| { *c1.lock().unwrap() += 1; });
    runtime.subscribe("broadcast", move |_| { *c2.lock().unwrap() += 1; });

    runtime.emit_event("broadcast", b"announcement");

    assert_eq!(*sub1_count.lock().unwrap(), 1);
    assert_eq!(*sub2_count.lock().unwrap(), 1);
}

#[test]
fn test_r71_4_e_concurrent_subscriber_thread_safety() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let seed = [0x9Au8; 32];
    let runtime = Arc::new(RuntimeInstance::new(5, temp_dir.path().to_path_buf(), seed).expect("Runtime creation failed"));

    let mut handles = Vec::new();
    for thread_idx in 0..10 {
        let r_clone = Arc::clone(&runtime);
        handles.push(std::thread::spawn(move || {
            let topic = format!("channel_{}", thread_idx % 3);
            let sub_id = r_clone.subscribe(&topic, move |_| {});
            r_clone.emit_event(&topic, b"data");
            r_clone.unsubscribe(sub_id);
        }));
    }

    for h in handles {
        h.join().expect("Worker thread failed");
    }
}

#[test]
fn test_r71_4_f_zero_subscriber_no_op() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let seed = [0xBCu8; 32];
    let runtime = RuntimeInstance::new(6, temp_dir.path().to_path_buf(), seed).expect("Runtime creation failed");

    // Emitting to empty topic must be a clean no-op and never panic
    runtime.emit_event("empty/topic", b"orphan payload");
}
