use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::Arc;
use crate::ffi::handle::{
    HandleRegistry, RuntimeInstance, NEX_ABI_VERSION_1, NEX_SUCCESS,
    NEX_ERR_INVALID_HANDLE, NEX_ERR_INTERNAL_ERROR, NEX_ERR_CAS_CORRUPTION,
};
use crate::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};
use crate::apps::drive::CasChunkStore;
use sha2::{Sha256, Digest};

// Simulated / Standard JNI Types for Pure Rust & FFI Compilation
#[repr(C)]
pub struct JNIEnv;

#[repr(C)]
pub struct JObject;

pub type JLong = i64;
pub type JInt = i32;
pub type JString = *const c_char;
pub type JByteArray = *const u8;

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCoreNative_nativeAbiVersion(
    _env: *mut JNIEnv,
    _class: *mut JObject,
) -> JInt {
    NEX_ABI_VERSION_1 as JInt
}

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCoreNative_nativeInit(
    _env: *mut JNIEnv,
    _class: *mut JObject,
    data_dir_utf8: *const c_char,
    master_seed_32: *const u8,
) -> JLong {
    let result = catch_unwind(|| {
        if data_dir_utf8.is_null() || master_seed_32.is_null() {
            return 0i64;
        }
        let c_str = unsafe { CStr::from_ptr(data_dir_utf8) };
        let dir_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return 0i64,
        };
        let mut seed = [0u8; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(master_seed_32, seed.as_mut_ptr(), 32);
        }

        let handle_id = HandleRegistry::allocate_id();
        let instance = match RuntimeInstance::new(handle_id, PathBuf::from(dir_str), seed) {
            Ok(inst) => inst,
            Err(_) => return 0i64,
        };

        HandleRegistry::register(instance);
        handle_id as JLong
    });

    result.unwrap_or(0i64)
}

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCoreNative_nativeStart(
    _env: *mut JNIEnv,
    _class: *mut JObject,
    handle_id: JLong,
) -> JInt {
    let result = catch_unwind(|| {
        if handle_id <= 0 {
            return NEX_ERR_INVALID_HANDLE;
        }
        let instance = match HandleRegistry::get(handle_id as u64) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };
        instance.start()
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCoreNative_nativeStop(
    _env: *mut JNIEnv,
    _class: *mut JObject,
    handle_id: JLong,
) -> JInt {
    let result = catch_unwind(|| {
        if handle_id <= 0 {
            return NEX_ERR_INVALID_HANDLE;
        }
        let instance = match HandleRegistry::get(handle_id as u64) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };
        instance.stop()
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCoreNative_nativeFree(
    _env: *mut JNIEnv,
    _class: *mut JObject,
    handle_id: JLong,
) {
    let _ = catch_unwind(|| {
        if handle_id > 0 {
            HandleRegistry::remove(handle_id as u64);
        }
    });
}

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCoreNative_nativeInvoke(
    _env: *mut JNIEnv,
    _class: *mut JObject,
    handle_id: JLong,
    req_bytes: *const u8,
    req_len: usize,
    resp_vec_out: *mut Vec<u8>,
) -> JInt {
    let result = catch_unwind(|| {
        if handle_id <= 0 || req_bytes.is_null() || resp_vec_out.is_null() {
            return NEX_ERR_INVALID_HANDLE;
        }
        let instance = match HandleRegistry::get(handle_id as u64) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };

        let req_slice = unsafe { std::slice::from_raw_parts(req_bytes, req_len) };
        let req_str = match std::str::from_utf8(req_slice) {
            Ok(s) => s,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };

        let req_obj: JsonRpcRequest = match serde_json::from_str(req_str) {
            Ok(v) => v,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };

        let mut node_guard = match instance.node.lock() {
            Ok(g) => g,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };

        let resp_obj = NexRpcDispatcher::dispatch_node(&mut *node_guard, req_obj);
        let resp_string = match serde_json::to_string(&resp_obj) {
            Ok(s) => s,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };

        unsafe {
            *resp_vec_out = resp_string.into_bytes();
        }

        NEX_SUCCESS
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCasStreamNative_nativePutChunkDirect(
    _env: *mut JNIEnv,
    _class: *mut JObject,
    handle_id: JLong,
    expected_digest_32: *const u8,
    direct_buffer_ptr: *const u8,
    buffer_len: usize,
) -> JInt {
    let result = catch_unwind(|| {
        if handle_id <= 0 || expected_digest_32.is_null() || direct_buffer_ptr.is_null() {
            return NEX_ERR_INVALID_HANDLE;
        }
        let instance = match HandleRegistry::get(handle_id as u64) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };

        let mut expected = [0u8; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(expected_digest_32, expected.as_mut_ptr(), 32);
        }

        let slice = unsafe { std::slice::from_raw_parts(direct_buffer_ptr, buffer_len) };
        let actual_digest: [u8; 32] = Sha256::digest(slice).into();

        if actual_digest != expected {
            return NEX_ERR_CAS_CORRUPTION;
        }

        let mut node_guard = match instance.node.lock() {
            Ok(g) => g,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };

        node_guard.storage.cas.chunks.insert(actual_digest, slice.to_vec());
        NEX_SUCCESS
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}

#[no_mangle]
pub extern "system" fn Java_app_nex_core_NexCasStreamNative_nativeGetChunkDirect(
    _env: *mut JNIEnv,
    _class: *mut JObject,
    handle_id: JLong,
    digest_32: *const u8,
    direct_buffer_out: *mut u8,
    max_len: usize,
) -> JInt {
    let result = catch_unwind(|| {
        if handle_id <= 0 || digest_32.is_null() || direct_buffer_out.is_null() {
            return NEX_ERR_INVALID_HANDLE;
        }
        let instance = match HandleRegistry::get(handle_id as u64) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };

        let mut digest = [0u8; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(digest_32, digest.as_mut_ptr(), 32);
        }

        let node_guard = match instance.node.lock() {
            Ok(g) => g,
            Err(_) => return NEX_ERR_INTERNAL_ERROR,
        };

        match node_guard.storage.cas.get_chunk(&digest) {
            Some(data) => {
                if data.len() > max_len {
                    return NEX_ERR_INTERNAL_ERROR;
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr(), direct_buffer_out, data.len());
                }
                data.len() as JInt
            }
            None => NEX_ERR_CAS_CORRUPTION,
        }
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}
