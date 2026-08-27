use std::ffi::CString;
use tempfile::tempdir;
use nex_core::ffi::jni_bridge::*;
use nex_core::ffi::handle::*;
use sha2::{Sha256, Digest};

#[test]
fn test_r57_1_a_jni_abi_version_handshake() {
    let version = Java_app_nex_core_NexCoreNative_nativeAbiVersion(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    assert_eq!(version, NEX_ABI_VERSION_1 as JInt);
}

#[test]
fn test_r57_1_b_jni_handle_lifecycle() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [101u8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        dir_c.as_ptr(),
        seed.as_ptr(),
    );
    assert!(handle_id > 0, "Handle ID must be positive");

    assert_eq!(
        Java_app_nex_core_NexCoreNative_nativeStart(std::ptr::null_mut(), std::ptr::null_mut(), handle_id),
        NEX_SUCCESS
    );

    assert_eq!(
        Java_app_nex_core_NexCoreNative_nativeStop(std::ptr::null_mut(), std::ptr::null_mut(), handle_id),
        NEX_SUCCESS
    );

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r57_1_c_jni_synchronous_invocation() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [102u8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        dir_c.as_ptr(),
        seed.as_ptr(),
    );
    assert_eq!(
        Java_app_nex_core_NexCoreNative_nativeStart(std::ptr::null_mut(), std::ptr::null_mut(), handle_id),
        NEX_SUCCESS
    );

    let req_json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "nex_ping",
        "params": {}
    }).to_string();

    let mut resp_vec = Vec::new();
    let status = Java_app_nex_core_NexCoreNative_nativeInvoke(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        req_json.as_ptr(),
        req_json.len(),
        &mut resp_vec,
    );

    assert_eq!(status, NEX_SUCCESS);
    assert!(!resp_vec.is_empty());

    let resp_val: serde_json::Value = serde_json::from_slice(&resp_vec).unwrap();
    assert_eq!(resp_val["jsonrpc"], "2.0");
    assert_eq!(resp_val["result"]["status"], "pong");

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r57_1_d_direct_byte_buffer_cas_streaming() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [103u8; 32];

    let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        dir_c.as_ptr(),
        seed.as_ptr(),
    );
    Java_app_nex_core_NexCoreNative_nativeStart(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);

    let payload = b"DirectByteBuffer zero copy off-heap chunk stream bytes";
    let digest: [u8; 32] = Sha256::digest(payload).into();

    // Put Chunk Direct
    let put_status = Java_app_nex_core_NexCasStreamNative_nativePutChunkDirect(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        digest.as_ptr(),
        payload.as_ptr(),
        payload.len(),
    );
    assert_eq!(put_status, NEX_SUCCESS);

    // Get Chunk Direct
    let mut out_buffer = vec![0u8; 256];
    let bytes_read = Java_app_nex_core_NexCasStreamNative_nativeGetChunkDirect(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        handle_id,
        digest.as_ptr(),
        out_buffer.as_mut_ptr(),
        out_buffer.len(),
    );

    assert_eq!(bytes_read as usize, payload.len());
    assert_eq!(&out_buffer[0..bytes_read as usize], payload);

    Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
}

#[test]
fn test_r57_1_e_jni_panic_boundary_isolation() {
    // 1. Invocation with invalid handle
    let mut resp_vec = Vec::new();
    let req = b"{}";
    assert_eq!(
        Java_app_nex_core_NexCoreNative_nativeInvoke(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            -999,
            req.as_ptr(),
            req.len(),
            &mut resp_vec
        ),
        NEX_ERR_INVALID_HANDLE
    );

    // 2. Put chunk with null buffer
    assert_eq!(
        Java_app_nex_core_NexCasStreamNative_nativePutChunkDirect(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            -999,
            std::ptr::null(),
            std::ptr::null(),
            0
        ),
        NEX_ERR_INVALID_HANDLE
    );
}

#[test]
fn test_r57_1_f_zero_regression_across_jni_lifecycle() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [104u8; 32];

    for _ in 0..5 {
        let handle_id = Java_app_nex_core_NexCoreNative_nativeInit(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            dir_c.as_ptr(),
            seed.as_ptr(),
        );
        assert_eq!(
            Java_app_nex_core_NexCoreNative_nativeStart(std::ptr::null_mut(), std::ptr::null_mut(), handle_id),
            NEX_SUCCESS
        );
        assert_eq!(
            Java_app_nex_core_NexCoreNative_nativeStop(std::ptr::null_mut(), std::ptr::null_mut(), handle_id),
            NEX_SUCCESS
        );
        Java_app_nex_core_NexCoreNative_nativeFree(std::ptr::null_mut(), std::ptr::null_mut(), handle_id);
    }
}
