use std::ffi::{CStr, c_char};
use std::collections::HashSet;
use std::slice;
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::{RwLock, OnceLock};
use crate::ffi::handle::{
    HandleRegistry, RuntimeInstance, NEX_ABI_VERSION_1, NEX_SUCCESS,
    NEX_ERR_INVALID_HANDLE, NEX_ERR_INTERNAL_ERROR, NEX_ERR_OBJECT_NOT_FOUND,
};
use crate::ipc::rpc::{NexRpcDispatcher, JsonRpcRequest};

#[repr(C)]
pub struct NexHandle {
    pub handle_id: u64,
}

static ACTIVE_HANDLE_POINTERS: OnceLock<RwLock<HashSet<usize>>> = OnceLock::new();

fn get_active_pointers() -> &'static RwLock<HashSet<usize>> {
    ACTIVE_HANDLE_POINTERS.get_or_init(|| RwLock::new(HashSet::new()))
}

fn is_valid_pointer(ptr: *mut NexHandle) -> bool {
    if ptr.is_null() {
        return false;
    }
    let set = get_active_pointers();
    if let Ok(guard) = set.read() {
        guard.contains(&(ptr as usize))
    } else {
        false
    }
}

fn register_pointer(ptr: *mut NexHandle) {
    let set = get_active_pointers();
    if let Ok(mut guard) = set.write() {
        guard.insert(ptr as usize);
    }
}

fn unregister_pointer(ptr: *mut NexHandle) -> bool {
    let set = get_active_pointers();
    if let Ok(mut guard) = set.write() {
        guard.remove(&(ptr as usize))
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn nex_abi_version() -> u32 {
    NEX_ABI_VERSION_1
}

#[no_mangle]
pub extern "C" fn nex_runtime_init(
    data_dir_utf8: *const c_char,
    master_seed_32: *const u8,
) -> *mut NexHandle {
    let result = catch_unwind(|| {
        if data_dir_utf8.is_null() || master_seed_32.is_null() {
            return std::ptr::null_mut();
        }
        let c_str = unsafe { CStr::from_ptr(data_dir_utf8) };
        let dir_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let mut seed = [0u8; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(master_seed_32, seed.as_mut_ptr(), 32);
        }

        let handle_id = HandleRegistry::allocate_id();
        let instance = match RuntimeInstance::new(handle_id, PathBuf::from(dir_str), seed) {
            Ok(inst) => inst,
            Err(_) => return std::ptr::null_mut(),
        };

        HandleRegistry::register(instance);
        let ptr = Box::into_raw(Box::new(NexHandle { handle_id }));
        register_pointer(ptr);
        ptr
    });

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn nex_runtime_start(handle: *mut NexHandle) -> i32 {
    let result = catch_unwind(|| {
        if !is_valid_pointer(handle) {
            return NEX_ERR_INVALID_HANDLE;
        }
        let handle_id = unsafe { (*handle).handle_id };
        let instance = match HandleRegistry::get(handle_id) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };
        instance.start()
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}

#[no_mangle]
pub extern "C" fn nex_runtime_stop(handle: *mut NexHandle) -> i32 {
    let result = catch_unwind(|| {
        if !is_valid_pointer(handle) {
            return NEX_ERR_INVALID_HANDLE;
        }
        let handle_id = unsafe { (*handle).handle_id };
        let instance = match HandleRegistry::get(handle_id) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };
        instance.stop()
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}

#[no_mangle]
pub extern "C" fn nex_runtime_free(handle: *mut NexHandle) {
    let _ = catch_unwind(|| {
        if !unregister_pointer(handle) {
            // Pointer is null, already freed, or unregistered -> Safe No-Op!
            return;
        }
        let handle_id = unsafe { (*handle).handle_id };
        HandleRegistry::remove(handle_id);
        unsafe {
            drop(Box::from_raw(handle));
        }
    });
}

#[no_mangle]
pub extern "C" fn nex_runtime_invoke(
    handle: *mut NexHandle,
    req_bytes: *const u8,
    req_len: usize,
    resp_bytes_out: *mut *mut u8,
    resp_len_out: *mut usize,
) -> i32 {
    let result = catch_unwind(|| {
        if !is_valid_pointer(handle) || req_bytes.is_null() || resp_bytes_out.is_null() || resp_len_out.is_null() {
            return NEX_ERR_INVALID_HANDLE;
        }
        let handle_id = unsafe { (*handle).handle_id };
        let instance = match HandleRegistry::get(handle_id) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };

        let req_slice = unsafe { slice::from_raw_parts(req_bytes, req_len) };
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

        let mut resp_vec = resp_string.into_bytes();
        resp_vec.shrink_to_fit();

        let len = resp_vec.len();
        let ptr = resp_vec.as_mut_ptr();
        std::mem::forget(resp_vec);

        unsafe {
            *resp_bytes_out = ptr;
            *resp_len_out = len;
        }

        NEX_SUCCESS
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}

#[no_mangle]
pub extern "C" fn nex_buffer_free(buffer: *mut u8, len: usize) {
    let _ = catch_unwind(|| {
        if !buffer.is_null() && len > 0 {
            unsafe {
                let _ = Vec::from_raw_parts(buffer, len, len);
            }
        }
    });
}

pub type NexEventCallback = extern "C" fn(*const u8, usize, *mut std::ffi::c_void);

#[no_mangle]
pub extern "C" fn nex_runtime_subscribe(
    handle: *mut NexHandle,
    topic_utf8: *const c_char,
    callback: NexEventCallback,
    user_data: *mut std::ffi::c_void,
) -> i64 {
    let result = catch_unwind(|| {
        if !is_valid_pointer(handle) || topic_utf8.is_null() {
            return -1i64;
        }
        let handle_id = unsafe { (*handle).handle_id };
        let instance = match HandleRegistry::get(handle_id) {
            Some(inst) => inst,
            None => return -1i64,
        };

        let c_str = unsafe { CStr::from_ptr(topic_utf8) };
        let topic = match c_str.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1i64,
        };

        let user_data_usize = user_data as usize;
        let sub_id = instance.subscribe(&topic, move |payload| {
            callback(payload.as_ptr(), payload.len(), user_data_usize as *mut std::ffi::c_void);
        });

        sub_id as i64
    });

    result.unwrap_or(-1i64)
}

#[no_mangle]
pub extern "C" fn nex_runtime_unsubscribe(handle: *mut NexHandle, sub_id: u64) -> i32 {
    let result = catch_unwind(|| {
        if !is_valid_pointer(handle) {
            return NEX_ERR_INVALID_HANDLE;
        }
        let handle_id = unsafe { (*handle).handle_id };
        let instance = match HandleRegistry::get(handle_id) {
            Some(inst) => inst,
            None => return NEX_ERR_INVALID_HANDLE,
        };
        if instance.unsubscribe(sub_id) {
            NEX_SUCCESS
        } else {
            NEX_ERR_OBJECT_NOT_FOUND
        }
    });

    result.unwrap_or(NEX_ERR_INTERNAL_ERROR)
}
