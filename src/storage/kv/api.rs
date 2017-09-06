use std;
use std::os::raw::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use futures::Future;
use storage;
use super::KVStorage;

type KVStorageType = KVStorage + Send;

#[derive(Clone)]
pub struct KVStorageHandle {
    inner: Arc<Mutex<KVStorageType>>
}

impl Deref for KVStorageHandle {
    type Target = Mutex<KVStorage>;
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
    let t: Arc<Mutex<KVStorageType>> = Arc::new(Mutex::new(storage::backend::redis::RedisStorage::new(
        CStr::from_ptr(conn_str).to_str().unwrap()
    )));
    Box::into_raw(Box::new(KVStorageHandle {
        inner: t
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
    let k = CStr::from_ptr(k).to_str().unwrap().to_string();

    storage::executor::get_event_loop().spawn(move |_| {
        let handle = handle.lock().unwrap();
        Box::new(handle.get(k.as_str())
            .map(move |v| {
                let result = match v {
                    Some(v) => Some(CString::new(v).unwrap()),
                    None => None
                };
                cb(call_with, match result {
                    Some(v) => v.as_ptr(),
                    None => std::ptr::null()
                });
                ()
            })
            .map_err(move |_| cb(call_with, std::ptr::null())))
    });
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
    let k = CStr::from_ptr(k).to_str().unwrap().to_string();
    let v = CStr::from_ptr(v).to_str().unwrap().to_string();

    storage::executor::get_event_loop().spawn(move |_| {
        let handle = handle.lock().unwrap();
        Box::new(handle.set(k.as_str(), v.as_str())
            .map(move |_| cb(call_with))
            .map_err(move |_| cb(call_with)))
    });
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
    let k = CStr::from_ptr(k).to_str().unwrap().to_string();

    storage::executor::get_event_loop().spawn(move |_| {
        let handle = handle.lock().unwrap();
        Box::new(handle.remove(k.as_str())
            .map(move |_| cb(call_with))
            .map_err(move |_| cb(call_with)))
    });
}
