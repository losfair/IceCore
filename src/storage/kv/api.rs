use std;
use std::os::raw::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use futures::Future;
use storage;
use super::{KVStorage, HashMapExtContainer};

type KVStorageType = KVStorage + Send + Sync;

#[derive(Clone)]
pub struct KVStorageHandle {
    inner: Arc<KVStorageType>
}

impl Deref for KVStorageHandle {
    type Target = KVStorageType;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

type GetItemCallbackFn = extern fn (usize, *const c_char);
type SetItemCallbackFn = extern fn (usize);
type RemoveItemCallbackFn = extern fn (usize);

#[no_mangle]
pub unsafe fn ice_storage_kv_create_with_redis_backend(
    conn_str: *const c_char
) -> *mut KVStorageHandle {
    Box::into_raw(Box::new(KVStorageHandle {
        inner: Arc::new(storage::backend::redis::RedisStorage::new(
            CStr::from_ptr(conn_str).to_str().unwrap()
        ))
    }))
}

#[no_mangle]
pub unsafe fn ice_storage_kv_destroy(handle: *mut KVStorageHandle) {
    Box::from_raw(handle);
}

#[no_mangle]
pub unsafe fn ice_storage_kv_get(
    handle: *mut KVStorageHandle,
    k: *const c_char,
    cb: GetItemCallbackFn,
    call_with: *const c_void
) {
    let handle = &*handle;
    let handle = handle.clone();
    let call_with = call_with as usize;
    let k = CStr::from_ptr(k).to_str().unwrap();

    let f = handle.get(k)
        .map(move |v| {
            match v {
                Some(v) => {
                    let v = CString::new(v).unwrap();
                    cb(call_with, v.as_ptr())
                },
                None => cb(call_with, std::ptr::null())
            }
            ()
        })
        .map_err(move |_| cb(call_with, std::ptr::null()));

    storage::executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe fn ice_storage_kv_set(
    handle: *mut KVStorageHandle,
    k: *const c_char,
    v: *const c_char,
    cb: SetItemCallbackFn,
    call_with: *const c_void
) {
    let handle = &*handle;
    let handle = handle.clone();
    let call_with = call_with as usize;
    let k = CStr::from_ptr(k).to_str().unwrap();
    let v = CStr::from_ptr(v).to_str().unwrap();

    let f = handle.set(k, v)
        .map(move |_| cb(call_with))
        .map_err(move |_| cb(call_with));

    storage::executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe fn ice_storage_kv_remove(
    handle: *mut KVStorageHandle,
    k: *const c_char,
    cb: RemoveItemCallbackFn,
    call_with: *const c_void
) {
    let handle = &*handle;
    let handle = handle.clone();
    let call_with = call_with as usize;
    let k = CStr::from_ptr(k).to_str().unwrap();

    let f = handle.remove(k)
        .map(move |_| cb(call_with))
        .map_err(move |_| cb(call_with));

    storage::executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe fn ice_storage_kv_expire_sec(
    handle: *mut KVStorageHandle,
    k: *const c_char,
    t: u32,
    cb: SetItemCallbackFn,
    call_with: *const c_void
) {
    let handle = &*handle;
    let handle = handle.clone();
    let call_with = call_with as usize;
    let k = CStr::from_ptr(k).to_str().unwrap();

    let f = handle.expire_sec(k, t)
        .map(move |_| cb(call_with))
        .map_err(move |_| cb(call_with));

    storage::executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe fn ice_storage_kv_get_hash_map_ext(
    handle: *mut KVStorageHandle
) -> *const HashMapExtContainer {
    let handle = &*handle;
    match handle.get_hash_map_ext() {
        Some(v) => v,
        None => std::ptr::null()
    }
}

#[no_mangle]
pub unsafe fn ice_storage_kv_hash_map_ext_get(
    target: *const HashMapExtContainer,
    k: *const c_char,
    map_key: *const c_char,
    cb: GetItemCallbackFn,
    call_with: *const c_void
) {
    let target = &*target;

    let call_with = call_with as usize;
    let k = CStr::from_ptr(k).to_str().unwrap();
    let map_key = CStr::from_ptr(map_key).to_str().unwrap();

    let f = target.get(k, map_key)
        .map(move |v| {
            match v {
                Some(v) => {
                    let v = CString::new(v).unwrap();
                    cb(call_with, v.as_ptr())
                },
                None => cb(call_with, std::ptr::null())
            }
            ()
        })
        .map_err(move |_| cb(call_with, std::ptr::null()));
        
    storage::executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe fn ice_storage_kv_hash_map_ext_set(
    target: *const HashMapExtContainer,
    k: *const c_char,
    map_key: *const c_char,
    v: *const c_char,
    cb: SetItemCallbackFn,
    call_with: *const c_void
) {
    let target = &*target;

    let call_with = call_with as usize;
    let k = CStr::from_ptr(k).to_str().unwrap();
    let map_key = CStr::from_ptr(map_key).to_str().unwrap();
    let v = CStr::from_ptr(v).to_str().unwrap();

    let f = target.set(k, map_key, v)
        .map(move |_| cb(call_with))
        .map_err(move |_| cb(call_with));
        
    storage::executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe fn ice_storage_kv_hash_map_ext_remove(
    target: *const HashMapExtContainer,
    k: *const c_char,
    map_key: *const c_char,
    cb: RemoveItemCallbackFn,
    call_with: *const c_void
) {
    let target = &*target;

    let call_with = call_with as usize;
    let k = CStr::from_ptr(k).to_str().unwrap();
    let map_key = CStr::from_ptr(map_key).to_str().unwrap();

    let f = target.remove(k, map_key)
        .map(move |_| cb(call_with))
        .map_err(move |_| cb(call_with));
        
    storage::executor::get_event_loop().spawn(move |_| f);
}
