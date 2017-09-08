extern crate hyper;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate uuid;
extern crate chrono;
extern crate tera;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_json;
/*
#[macro_use]
extern crate serde_derive;
*/
extern crate ansi_term;
extern crate etag;
extern crate sequence_trie;
extern crate byteorder;
extern crate net2;
extern crate num_cpus;
extern crate url;
extern crate redis;
extern crate threadpool;
extern crate r2d2;
extern crate r2d2_redis;
extern crate rand;

#[cfg(feature = "use_cervus")]
extern crate cervus;

mod ice_server;
mod delegates;
mod router;
pub mod glue;
mod config;
mod static_file;
mod session_storage;
mod time;
mod template;
mod logging;
mod stat;
pub mod streaming;
pub mod ext;
mod prefix_tree;
mod session_backends;
pub mod storage;
pub mod stream;
mod trait_handle;
mod executor;

#[cfg(test)]
mod prefix_tree_test;

use std::any::Any;
use std::sync::{Arc, Mutex, RwLock};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::borrow::BorrowMut;
use ice_server::IceServer;
use delegates::{ServerHandle, ContextHandle};

#[no_mangle]
pub fn ice_create_server() -> ServerHandle {
    let server = Arc::new(Mutex::new(IceServer::new()));
    Arc::into_raw(server)
}

#[no_mangle]
pub unsafe fn ice_server_listen(handle: ServerHandle, addr: *const c_char) -> *mut std::thread::JoinHandle<()> {
    let handle = &*handle;

    let server = handle.lock().unwrap();
    server.listen(CStr::from_ptr(addr).to_str().unwrap());

    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe fn ice_server_router_add_endpoint(handle: ServerHandle, p: *const c_char) -> *const RwLock<router::Endpoint> {
    let handle = &*handle;

    let server = handle.lock().unwrap();
    let mut router = server.prep.router.write().unwrap();
    let ep = router.add_endpoint(CStr::from_ptr(p).to_str().unwrap());

    Arc::into_raw(ep)
}

#[no_mangle]
pub unsafe fn ice_server_set_session_cookie_name(handle: ServerHandle, name: *const c_char) {
    let handle = &*handle;

    let mut server = handle.lock().unwrap();
    *server.prep.session_cookie_name.lock().unwrap() = CStr::from_ptr(name).to_str().unwrap().to_string();
}

#[no_mangle]
pub unsafe fn ice_server_set_session_timeout_ms(handle: ServerHandle, t: u64) {
    let handle = &*handle;

    let mut server = handle.lock().unwrap();
    *server.prep.session_timeout_ms.write().unwrap() = t;
}

#[no_mangle]
pub unsafe fn ice_server_add_template(handle: ServerHandle, name: *const c_char, content: *const c_char) -> bool {
    let handle = &*handle;

    let server = handle.lock().unwrap();
    let ret = server.prep.templates.add(
        CStr::from_ptr(name).to_str().unwrap(),
        CStr::from_ptr(content).to_str().unwrap()
    );

    ret
}

#[no_mangle]
pub unsafe fn ice_server_set_max_request_body_size(handle: ServerHandle, size: u32) {
    let handle = &*handle;

    let mut server = handle.lock().unwrap();
    *server.prep.max_request_body_size.lock().unwrap() = size;
}

#[no_mangle]
pub unsafe fn ice_server_disable_request_logging(handle: ServerHandle) {
    let handle = &*handle;

    let mut server = handle.lock().unwrap();
    *server.prep.log_requests.lock().unwrap() = false;
}

#[no_mangle]
pub unsafe fn ice_server_set_async_endpoint_cb(handle: ServerHandle, cb: extern fn (i32, *mut delegates::CallInfo)) {
    let handle = &*handle;

    let mut server = handle.lock().unwrap();
    *server.prep.async_endpoint_cb.lock().unwrap() = Some(cb);
}

#[no_mangle]
pub unsafe fn ice_server_set_endpoint_timeout_ms(handle: ServerHandle, t: u64) {
    let handle = &*handle;

    let mut server = handle.lock().unwrap();
    *server.prep.endpoint_timeout_ms.lock().unwrap() = t;
}

#[no_mangle]
pub unsafe fn ice_server_set_custom_app_data(handle: ServerHandle, ptr: *const c_void) {
    let handle = &*handle;

    let server = handle.lock().unwrap();
    server.prep.custom_app_data.set_raw(ptr);
}

#[no_mangle]
pub unsafe fn ice_server_cervus_load_bitcode(handle: ServerHandle, name: *const c_char, data: *const u8, data_len: u32) -> bool {
    let handle = &*handle;
    let server = handle.lock().unwrap();

    let name = CStr::from_ptr(name).to_str().unwrap();
    let data = std::slice::from_raw_parts(data, data_len as usize).to_vec();

    server.load_module(name, data.as_slice());
    true
}

#[no_mangle]
pub unsafe fn ice_server_use_redis_session_storage(handle: ServerHandle, conn_str: *const c_char) {
    let handle = &*handle;
    let server = handle.lock().unwrap();

    let conn_str = CStr::from_ptr(conn_str).to_str().unwrap();

    let mut session_storage = server.prep.session_storage.lock().unwrap();

    let executor = Box::new(tokio_core::reactor::Core::new().unwrap());
    *session_storage = Some(
        Arc::new(
            Box::new(
                session_backends::redis::RedisStorage::new(executor.remote(), conn_str, *server.prep.session_timeout_ms.read().unwrap())
            ).into()
        )
    );

    let executor: usize = Box::into_raw(executor) as usize;
    std::thread::spawn(move || {
        let executor = executor as *mut tokio_core::reactor::Core;
        let mut executor = Box::from_raw(executor);
        executor.run(futures::future::empty::<(), ()>()).unwrap();
    });
}

#[no_mangle]
pub unsafe fn ice_context_render_template(handle: ContextHandle, name: *const c_char, data: *const c_char) -> *mut c_char {
    let handle = &*handle;

    let ret = match handle.templates.render_json(
        CStr::from_ptr(name).to_str().unwrap(),
        CStr::from_ptr(data).to_str().unwrap()
    ) {
        Some(v) => CString::new(v).unwrap().into_raw(),
        None => std::ptr::null_mut()
    };

    ret
}

#[no_mangle]
pub unsafe fn ice_context_get_stats(handle: ContextHandle) -> *mut c_char {
    let handle = &*handle;

    let ret = CString::new(handle.stats.serialize().to_string()).unwrap().into_raw();

    ret
}

#[no_mangle]
pub unsafe fn ice_context_stats_set_custom(handle: ContextHandle, k: *const c_char, v: *const c_char) {
    let handle = &*handle;

    let k = CStr::from_ptr(k).to_str().unwrap().to_string();
    let v = CStr::from_ptr(v).to_str().unwrap().to_string();

    handle.stats.set_custom(k, v);
}

#[no_mangle]
pub unsafe fn ice_context_set_custom_app_data(handle: ContextHandle, ptr: *const c_void) {
    let handle = &*handle;

    handle.custom_app_data.set_raw(ptr);
}

#[cfg(feature = "cervus")]
#[no_mangle]
pub unsafe fn ice_context_get_service_handle_by_module_name_and_service_name(
    handle: ContextHandle,
    module_name: *const c_char,
    service_name: *const c_char
) -> *mut cervus::manager::ServiceHandle {
    let handle = &*handle;
    let module_name = CStr::from_ptr(module_name).to_str().unwrap();
    let service_name = CStr::from_ptr(service_name).to_str().unwrap();

    match handle.get_service_by_name(module_name, service_name) {
        Some(v) => Box::into_raw(Box::new(v)),
        None => std::ptr::null_mut()
    }
}

#[cfg(feature = "cervus")]
#[no_mangle]
pub unsafe fn ice_core_service_handle_call_with_raw_pointer(
    handle: *mut cervus::manager::ServiceHandle,
    ptr: *mut c_void
) -> *mut cervus::engine::ModuleResource {
    let handle = &*handle;
    let ret = match handle.call(Box::new(ptr) as Box<Any>) {
        Some(v) => v,
        None => return std::ptr::null_mut()
    };
    Box::into_raw(
        Box::new(
            cervus::engine::ModuleResource::from(ret)
        )
    )
}

#[cfg(feature = "cervus")]
#[no_mangle]
pub unsafe fn ice_core_destroy_module_resource(handle: *mut cervus::engine::ModuleResource) {
    Box::from_raw(handle);
}

#[cfg(feature = "cervus")]
#[no_mangle]
pub unsafe fn ice_core_destroy_service_handle(handle: *mut cervus::manager::ServiceHandle) {
    Box::from_raw(handle);
}

#[no_mangle]
pub unsafe fn ice_core_destroy_context_handle(handle: ContextHandle) {
    Arc::from_raw(handle);
}

#[no_mangle]
pub unsafe fn ice_core_fire_callback(call_info: *mut delegates::CallInfo, resp: *mut glue::response::Response) -> bool {
    let call_info = Box::from_raw(call_info);
    let resp = Box::from_raw(resp);

    match call_info.tx.send(resp) {
        Ok(_) => true,
        Err(_) => false
    }
}

#[no_mangle]
pub unsafe fn ice_core_borrow_request_from_call_info(call_info: *mut delegates::CallInfo) -> *mut glue::request::Request {
    let mut call_info = &mut *call_info;

    let req = call_info.req.borrow_mut() as *mut glue::request::Request;

    req
}

#[no_mangle]
pub unsafe fn ice_core_get_custom_app_data_from_call_info(call_info: *mut delegates::CallInfo) -> *const c_void {
    let call_info = &*call_info;

    call_info.custom_app_data.get_raw()
}

#[no_mangle]
pub unsafe fn ice_core_endpoint_get_id(ep: *const RwLock<router::Endpoint>) -> i32 {
    let ep = &*ep;
    ep.read().unwrap().id
}

#[no_mangle]
pub unsafe fn ice_core_endpoint_set_flag(ep: *const RwLock<router::Endpoint>, name: *const c_char, value: bool) {
    let ep = &*ep;
    ep.write().unwrap().flags.insert(CStr::from_ptr(name).to_str().unwrap().to_string(), value);
}

#[no_mangle]
pub unsafe fn ice_core_destroy_cstring(v: *mut c_char) {
    CString::from_raw(v);
}

#[no_mangle]
pub unsafe fn ice_core_cervus_enabled() -> bool {
    if cfg!(feature = "use_cervus") {
        true
    } else {
        false
    }
}
