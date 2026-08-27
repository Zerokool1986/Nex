use std::ffi::CString;
use tempfile::tempdir;
use nex_core::ffi::c_abi::*;
use nex_core::ffi::handle::*;

#[test]
fn test_r56_1_a_abi_version_handshake() {
    let version = nex_abi_version();
    assert_eq!(version, NEX_ABI_VERSION_1, "ABI version must match 0x00010000");
}

#[test]
fn test_r56_1_b_handle_lifecycle_and_double_free_immunity() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [42u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert!(!handle.is_null(), "Handle must not be null");

    let start_res = nex_runtime_start(handle);
    assert_eq!(start_res, NEX_SUCCESS, "Start must return SUCCESS");

    let stop_res = nex_runtime_stop(handle);
    assert_eq!(stop_res, NEX_SUCCESS, "Stop must return SUCCESS");

    // Free the handle
    nex_runtime_free(handle);

    // Double free must be a safe no-op (zero panic, zero segfault)
    nex_runtime_free(handle);
}

#[test]
fn test_r56_1_c_synchronous_command_invocation_over_c_abi() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [77u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert!(!handle.is_null());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    let req_json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "nex_ping",
        "params": {}
    }).to_string();

    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;

    let status = nex_runtime_invoke(
        handle,
        req_json.as_ptr(),
        req_json.len(),
        &mut resp_ptr,
        &mut resp_len,
    );

    assert_eq!(status, NEX_SUCCESS);
    assert!(!resp_ptr.is_null());
    assert!(resp_len > 0);

    let resp_slice = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let resp_str = std::str::from_utf8(resp_slice).unwrap();
    let resp_val: serde_json::Value = serde_json::from_str(resp_str).unwrap();

    assert_eq!(resp_val["jsonrpc"], "2.0");
    assert_eq!(resp_val["result"]["status"], "pong");

    // Deallocate response buffer
    nex_buffer_free(resp_ptr, resp_len);
    nex_runtime_free(handle);
}

#[test]
fn test_r56_1_d_drive_put_and_list_over_c_abi() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [88u8; 32];

    let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    // Drive Put
    let put_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "nex_drivePut",
        "params": {
            "path": "/docs/contract.txt",
            "payload": hex::encode(b"Confidential Agreement Terms")
        }
    }).to_string();

    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;

    assert_eq!(
        nex_runtime_invoke(handle, put_req.as_ptr(), put_req.len(), &mut resp_ptr, &mut resp_len),
        NEX_SUCCESS
    );
    nex_buffer_free(resp_ptr, resp_len);

    // Drive List
    let list_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "nex_driveList",
        "params": {
            "path": "/docs"
        }
    }).to_string();

    assert_eq!(
        nex_runtime_invoke(handle, list_req.as_ptr(), list_req.len(), &mut resp_ptr, &mut resp_len),
        NEX_SUCCESS
    );

    let resp_slice = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let resp_val: serde_json::Value = serde_json::from_slice(resp_slice).unwrap();
    let entries = resp_val["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["path"], "/docs/contract.txt");

    nex_buffer_free(resp_ptr, resp_len);
    nex_runtime_free(handle);
}

#[test]
fn test_r56_1_e_panic_boundary_isolation_and_null_safety() {
    // 1. Invocation with NULL handle
    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;
    let req = b"{}";

    let status = nex_runtime_invoke(
        std::ptr::null_mut(),
        req.as_ptr(),
        req.len(),
        &mut resp_ptr,
        &mut resp_len,
    );
    assert_eq!(status, NEX_ERR_INVALID_HANDLE);

    // 2. Start with NULL handle
    assert_eq!(nex_runtime_start(std::ptr::null_mut()), NEX_ERR_INVALID_HANDLE);

    // 3. Stop with NULL handle
    assert_eq!(nex_runtime_stop(std::ptr::null_mut()), NEX_ERR_INVALID_HANDLE);

    // 4. Free with NULL handle (safe no-op)
    nex_runtime_free(std::ptr::null_mut());
    nex_buffer_free(std::ptr::null_mut(), 0);
}

#[test]
fn test_r56_1_f_zero_regression_across_c_abi_lifecycle() {
    let dir = tempdir().unwrap();
    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let seed = [99u8; 32];

    for _ in 0..5 {
        let handle = nex_runtime_init(dir_c.as_ptr(), seed.as_ptr());
        assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);
        assert_eq!(nex_runtime_stop(handle), NEX_SUCCESS);
        nex_runtime_free(handle);
    }
}
