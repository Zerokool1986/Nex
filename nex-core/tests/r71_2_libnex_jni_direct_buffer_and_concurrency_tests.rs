use std::ffi::CString;
use tempfile::tempdir;
use nex_core::ffi::jni_bridge::*;
use nex_core::ffi::handle::*;

#[test]
fn test_r71_2_a_jni_abi_version_and_lifecycle() {
    let version = Java_app_nex_core_NexCoreNative_nativeAbiVersion(std::ptr::null_mut(), std::ptr::null_mut());
    assert_eq!(version, NEX_ABI_VERSION_1 as JInt);

    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0x77u8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        c_dir.as_ptr(),
        seed.as_ptr(),
    );
    assert!(handle_id > 0, "Valid handle ID must be returned");

    let start_res = Java_app_nex_core_NexCoreNative_nativeStart(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
    assert_eq!(start_res, NEX_SUCCESS);

    let stop_res = Java_app_nex_core_NexCoreNative_nativeStop(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
    assert_eq!(stop_res, NEX_SUCCESS);

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r71_2_b_jni_cas_chunk_direct_buffer_streaming() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0x88u8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        c_dir.as_ptr(),
        seed.as_ptr(),
    );
    assert!(handle_id > 0);
    assert_eq!(Java_app_nex_core_NexCoreNative_nativeStart(std::ptr::null_mut(), std::ptr::null_mut(), handle_id), NEX_SUCCESS);

    // 64 KB test chunk payload
    let chunk_data = vec![0x33u8; 64 * 1024];
    use sha2::{Sha256, Digest};
    let expected_hash: [u8; 32] = Sha256::digest(&chunk_data).into();

    let write_res = Java_app_nex_core_NexCasStreamNative_nativePutChunkDirect(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        expected_hash.as_ptr(),
        chunk_data.as_ptr(),
        chunk_data.len(),
    );
    assert_eq!(write_res, NEX_SUCCESS);

    // Read back through direct buffer
    let mut read_buf = vec![0u8; 64 * 1024];

    let read_res = Java_app_nex_core_NexCasStreamNative_nativeGetChunkDirect(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        expected_hash.as_ptr(),
        read_buf.as_mut_ptr(),
        read_buf.len(),
    );
    assert_eq!(read_res, (64 * 1024) as JInt);
    assert_eq!(read_buf, chunk_data);

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r71_2_c_jni_invalid_chunk_hash_rejection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0x99u8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        c_dir.as_ptr(),
        seed.as_ptr(),
    );
    assert!(handle_id > 0);

    let nonexistent_hash = [0xFFu8; 32];
    let mut read_buf = vec![0u8; 1024];

    let read_res = Java_app_nex_core_NexCasStreamNative_nativeGetChunkDirect(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        nonexistent_hash.as_ptr(),
        read_buf.as_mut_ptr(),
        read_buf.len(),
    );
    assert_eq!(read_res, NEX_ERR_CAS_CORRUPTION);

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r71_2_d_jni_concurrent_rpc_threads() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0xAAu8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        c_dir.as_ptr(),
        seed.as_ptr(),
    );
    assert!(handle_id > 0);
    assert_eq!(Java_app_nex_core_NexCoreNative_nativeStart(std::ptr::null_mut(), std::ptr::null_mut(), handle_id), NEX_SUCCESS);

    let mut handles = Vec::new();
    for thread_idx in 0..8 {
        handles.push(std::thread::spawn(move || {
            let req_json = format!(r#"{{"jsonrpc":"2.0","method":"system_health","params":{{"thread":{}}},"id":{}}}"#, thread_idx, thread_idx);
            let c_req = CString::new(req_json).unwrap();
            let mut out_vec = Vec::new();

            let status = Java_app_nex_core_NexCoreNative_nativeInvoke(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                handle_id,
                c_req.as_ptr() as *const u8,
                c_req.as_bytes().len(),
                &mut out_vec as *mut Vec<u8>,
            );
            assert_eq!(status, NEX_SUCCESS);
            assert!(!out_vec.is_empty());
        }));
    }

    for h in handles {
        h.join().expect("Worker thread panicked");
    }

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r71_2_e_jni_zero_copy_buffer_overflow_protection() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0xBBu8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        c_dir.as_ptr(),
        seed.as_ptr(),
    );

    let chunk_data = vec![0x44u8; 1000];
    use sha2::{Sha256, Digest};
    let expected_hash: [u8; 32] = Sha256::digest(&chunk_data).into();

    Java_app_nex_core_NexCasStreamNative_nativePutChunkDirect(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        expected_hash.as_ptr(),
        chunk_data.as_ptr(),
        chunk_data.len(),
    );

    // Client passes undersized buffer (500 bytes < 1000 bytes)
    let mut small_buf = vec![0u8; 500];

    let res = Java_app_nex_core_NexCasStreamNative_nativeGetChunkDirect(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        expected_hash.as_ptr(),
        small_buf.as_mut_ptr(),
        500,
    );
    assert_eq!(res, NEX_ERR_INTERNAL_ERROR);

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r71_2_f_jni_double_free_safety() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0xCCu8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        c_dir.as_ptr(),
        seed.as_ptr(),
    );

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}
