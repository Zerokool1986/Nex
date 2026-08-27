use std::ffi::CString;
use tempfile::tempdir;
use nex_core::ffi::c_abi::*;
use nex_core::ffi::handle::*;

#[test]
fn test_r71_1_a_c_abi_version_and_lifecycle_roundtrip() {
    assert_eq!(nex_abi_version(), NEX_ABI_VERSION_1);

    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0x42u8; 32];

    let handle = nex_runtime_init(c_dir.as_ptr(), seed.as_ptr());
    assert!(!handle.is_null(), "Handle must not be null upon successful initialization");

    let start_status = nex_runtime_start(handle);
    assert_eq!(start_status, NEX_SUCCESS);

    let stop_status = nex_runtime_stop(handle);
    assert_eq!(stop_status, NEX_SUCCESS);

    nex_runtime_free(handle);
}

#[test]
fn test_r71_1_b_invalid_pointer_and_double_free_rejection() {
    let invalid_ptr = 0xDEADBEEF as *mut NexHandle;
    assert_eq!(nex_runtime_start(invalid_ptr), NEX_ERR_INVALID_HANDLE);
    assert_eq!(nex_runtime_stop(invalid_ptr), NEX_ERR_INVALID_HANDLE);

    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0x55u8; 32];

    let handle = nex_runtime_init(c_dir.as_ptr(), seed.as_ptr());
    assert!(!handle.is_null());

    nex_runtime_free(handle);
    // Double free must be safely caught without panicking
    nex_runtime_free(handle);
}

#[test]
fn test_r71_1_c_rpc_invoke_null_checks_and_isolation() {
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let c_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();
    let seed = [0x66u8; 32];

    let handle = nex_runtime_init(c_dir.as_ptr(), seed.as_ptr());
    assert!(!handle.is_null());
    assert_eq!(nex_runtime_start(handle), NEX_SUCCESS);

    // Null JSON buffer check
    let mut out_len: usize = 0;
    let mut out_ptr: *mut u8 = std::ptr::null_mut();
    let status = nex_runtime_invoke(handle, std::ptr::null(), 0, &mut out_ptr, &mut out_len);
    assert_eq!(status, NEX_ERR_INVALID_HANDLE);

    // Valid ping RPC request
    let request_json = r#"{"jsonrpc":"2.0","method":"system_health","params":{},"id":1}"#;
    let c_req = CString::new(request_json).unwrap();
    let status_ok = nex_runtime_invoke(handle, c_req.as_ptr() as *const u8, request_json.len(), &mut out_ptr, &mut out_len);
    assert_eq!(status_ok, NEX_SUCCESS);
    assert!(!out_ptr.is_null());
    assert!(out_len > 0);

    // Free RPC response buffer
    nex_buffer_free(out_ptr, out_len);

    assert_eq!(nex_runtime_stop(handle), NEX_SUCCESS);
    nex_runtime_free(handle);
}

#[test]
fn test_r71_1_d_buffer_free_null_safety() {
    // Calling nex_buffer_free on null pointer must be a no-op and never panic
    nex_buffer_free(std::ptr::null_mut(), 0);
    nex_buffer_free(std::ptr::null_mut(), 1024);
}

#[test]
fn test_r71_1_e_multi_handle_allocation_isolation() {
    let temp_dir1 = tempdir().expect("Failed to create tempdir 1");
    let temp_dir2 = tempdir().expect("Failed to create tempdir 2");
    let c_dir1 = CString::new(temp_dir1.path().to_str().unwrap()).unwrap();
    let c_dir2 = CString::new(temp_dir2.path().to_str().unwrap()).unwrap();

    let seed1 = [0x11u8; 32];
    let seed2 = [0x22u8; 32];

    let handle1 = nex_runtime_init(c_dir1.as_ptr(), seed1.as_ptr());
    let handle2 = nex_runtime_init(c_dir2.as_ptr(), seed2.as_ptr());

    assert!(!handle1.is_null());
    assert!(!handle2.is_null());
    assert_ne!(handle1, handle2);

    assert_eq!(nex_runtime_start(handle1), NEX_SUCCESS);
    assert_eq!(nex_runtime_start(handle2), NEX_SUCCESS);

    // Free handle 1
    nex_runtime_free(handle1);

    // Handle 2 must remain fully functional
    assert_eq!(nex_runtime_stop(handle2), NEX_SUCCESS);
    nex_runtime_free(handle2);
}

#[test]
fn test_r71_1_f_panic_boundary_unwind_catch() {
    let bad_utf8 = [0xFF, 0xFE, 0x00];
    let seed = [0x33u8; 32];

    let handle = nex_runtime_init(bad_utf8.as_ptr() as *const std::os::raw::c_char, seed.as_ptr());
    assert!(handle.is_null(), "Malformed UTF-8 path must return null handle without panicking");
}
