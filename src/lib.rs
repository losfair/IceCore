extern crate hyper;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate uuid;
extern crate chrono;
extern crate tera;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate ansi_term;
extern crate etag;

mod ice_server;
mod delegates;
mod router;
mod glue;
mod config;
mod static_file;
mod session_storage;
mod time;
mod template;
mod logging;
mod stat;

use std::sync::{Arc, Mutex};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use ice_server::IceServer;
use delegates::{ServerHandle, SessionHandle, ContextHandle};

type Pointer = usize;

#[no_mangle]
pub fn ice_create_server() -> ServerHandle {
    Arc::into_raw(Arc::new(Mutex::new(IceServer::new())))
}

#[no_mangle]
pub fn ice_server_listen(handle: ServerHandle, addr: *const c_char) -> *mut std::thread::JoinHandle<()> {
    let handle = unsafe { Arc::from_raw(handle) };
    let thread_handle: Box<std::thread::JoinHandle<()>>;

    {
        let server = handle.lock().unwrap();
        thread_handle = Box::new(server.listen(unsafe { CStr::from_ptr(addr) }.to_str().unwrap()));
    }

    Arc::into_raw(handle);
    Box::into_raw(thread_handle)
}

#[no_mangle]
pub fn ice_server_router_add_endpoint(handle: ServerHandle, p: *const c_char) -> Pointer {
    let handle = unsafe { Arc::from_raw(handle) };
    let ep: Pointer;

    {
        let server = handle.lock().unwrap();
        let mut router = server.prep.router.lock().unwrap();
        ep = router.add_endpoint(unsafe { CStr::from_ptr(p) }.to_str().unwrap());
    }

    Arc::into_raw(handle);

    ep
}

#[no_mangle]
pub fn ice_server_set_static_dir(handle: ServerHandle, d: *const c_char) {
    let handle = unsafe { Arc::from_raw(handle) };

    {
        let mut server = handle.lock().unwrap();
        *server.prep.static_dir.write().unwrap() = Some(unsafe { CStr::from_ptr(d) }.to_str().unwrap().to_string());
    }

    Arc::into_raw(handle);
}

#[no_mangle]
pub fn ice_server_set_session_cookie_name(handle: ServerHandle, name: *const c_char) {
    let handle = unsafe { Arc::from_raw(handle) };

    {
        let mut server = handle.lock().unwrap();
        *server.prep.session_cookie_name.lock().unwrap() = unsafe { CStr::from_ptr(name) }.to_str().unwrap().to_string();
    }

    Arc::into_raw(handle);
}

#[no_mangle]
pub fn ice_server_set_session_timeout_ms(handle: ServerHandle, t: u64) {
    let handle = unsafe { Arc::from_raw(handle) };

    {
        let mut server = handle.lock().unwrap();
        *server.prep.session_timeout_ms.write().unwrap() = t;
    }

    Arc::into_raw(handle);
}

#[no_mangle]
pub fn ice_server_add_template(handle: ServerHandle, name: *const c_char, content: *const c_char) -> bool {
    let handle = unsafe { Arc::from_raw(handle) };
    let ret;

    {
        let server = handle.lock().unwrap();
        ret = server.prep.templates.add(
            unsafe { CStr::from_ptr(name) }.to_str().unwrap(),
            unsafe { CStr::from_ptr(content) }.to_str().unwrap()
        );
    }

    Arc::into_raw(handle);
    ret
}

#[no_mangle]
pub fn ice_server_set_max_request_body_size(handle: ServerHandle, size: u32) {
    let handle = unsafe { Arc::from_raw(handle) };

    {
        let mut server = handle.lock().unwrap();
        *server.prep.max_request_body_size.lock().unwrap() = size;
    }

    Arc::into_raw(handle);
}

#[no_mangle]
pub fn ice_context_render_template(handle: ContextHandle, name: *const c_char, data: *const c_char) -> *mut c_char {
    let handle = unsafe { Arc::from_raw(handle) };

    let ret = match handle.templates.render_json(
        unsafe { CStr::from_ptr(name) }.to_str().unwrap(),
        unsafe { CStr::from_ptr(data) }.to_str().unwrap()
    ) {
        Some(v) => CString::new(v).unwrap().into_raw(),
        None => std::ptr::null_mut()
    };

    Arc::into_raw(handle);
    ret
}

#[no_mangle]
pub fn ice_context_create_session(handle: ContextHandle) -> SessionHandle {
    let handle = unsafe { Arc::from_raw(handle) };

    let ret = Arc::into_raw(handle.session_storage.create_session());

    //println!("ice_context_create_session");

    Arc::into_raw(handle);
    ret
}

#[no_mangle]
pub fn ice_context_get_session_by_id(handle: ContextHandle, id: *const c_char) -> SessionHandle {
    let handle = unsafe { Arc::from_raw(handle) };
    let id = unsafe { CStr::from_ptr(id) }.to_str().unwrap();

    let ret = match handle.session_storage.get_session(id) {
        Some(v) => Arc::into_raw(v),
        None => std::ptr::null()
    };

    //println!("ice_context_get_session_by_id");

    Arc::into_raw(handle);
    ret
}

#[no_mangle]
pub fn ice_context_get_stats(handle: ContextHandle) -> *mut c_char {
    let handle = unsafe { Arc::from_raw(handle) };

    let ret = CString::new(handle.stats.serialize().to_string()).unwrap().into_raw();

    Arc::into_raw(handle);
    ret
}

#[no_mangle]
pub fn ice_context_stats_set_custom(handle: ContextHandle, k: *const c_char, v: *const c_char) {
    let handle = unsafe { Arc::from_raw(handle) };

    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap().to_string();
    let v = unsafe { CStr::from_ptr(v) }.to_str().unwrap().to_string();

    handle.stats.set_custom(k, v);

    Arc::into_raw(handle);
}

#[no_mangle]
pub fn ice_core_destroy_session_handle(handle: SessionHandle) {
    unsafe { Arc::from_raw(handle); }
    //println!("ice_core_destroy_session_handle");
}

#[no_mangle]
pub fn ice_core_session_get_id(handle: SessionHandle) -> *mut c_char {
    let handle = unsafe { Arc::from_raw(handle) };
    let ret = CString::new(handle.read().unwrap().get_id()).unwrap().into_raw();
    Arc::into_raw(handle);
    //println!("ice_core_session_get_id");
    ret
}

#[no_mangle]
pub fn ice_core_session_get_item(handle: SessionHandle, k: *const c_char) -> *mut c_char {
    let handle = unsafe { Arc::from_raw(handle) };
    let ret;

    {
        let sess = handle.read().unwrap();

        ret = match sess.data.get(&unsafe { CStr::from_ptr(k) }.to_str().unwrap().to_string()) {
            Some(v) => CString::new(v.as_str()).unwrap().into_raw(),
            None => std::ptr::null_mut()
        };
    }

    //println!("ice_core_session_get_item");

    Arc::into_raw(handle);
    ret
}

#[no_mangle]
pub fn ice_core_session_set_item(handle: SessionHandle, k: *const c_char, v: *const c_char) {
    let handle = unsafe { Arc::from_raw(handle) };

    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap().to_string();
    let v = unsafe { CStr::from_ptr(v) }.to_str().unwrap().to_string();

    handle.write().unwrap().data.insert(k, v);

    Arc::into_raw(handle);
}

#[no_mangle]
pub fn ice_core_session_remove_item(handle: SessionHandle, k: *const c_char) {
    let handle = unsafe { Arc::from_raw(handle) };

    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap().to_string();

    handle.write().unwrap().data.remove(&k);

    Arc::into_raw(handle);
}

#[no_mangle]
pub fn ice_core_destroy_context_handle(handle: ContextHandle) {
    unsafe { Arc::from_raw(handle); }
}

#[no_mangle]
pub fn ice_core_fire_callback(call_info: *mut delegates::CallInfo, resp: delegates::Pointer) {
    let call_info = unsafe { Box::from_raw(call_info) };

    call_info.tx.send(resp).unwrap();
}

#[no_mangle]
pub fn ice_core_borrow_request_from_call_info(call_info: *mut delegates::CallInfo) -> delegates::Pointer {
    let call_info = unsafe { Box::from_raw(call_info) };

    let raw_req = unsafe { call_info.req.get_raw() };

    Box::into_raw(call_info);

    raw_req
}

#[no_mangle]
pub fn ice_core_endpoint_get_id(ep: Pointer) -> i32 {
    unsafe { router::ice_internal_prefix_tree_endpoint_get_id(ep) }
}

#[no_mangle]
pub fn ice_core_endpoint_set_flag(ep: Pointer, name: *const c_char, value: bool) {
    unsafe { router::ice_internal_prefix_tree_endpoint_set_flag(ep, name, value); }
}

#[no_mangle]
pub fn ice_core_endpoint_get_flag(ep: Pointer, name: *const c_char) -> bool {
    unsafe { router::ice_internal_prefix_tree_endpoint_get_flag(ep, name) }
}

#[no_mangle]
pub fn ice_core_destroy_cstring(v: *mut c_char) {
    unsafe { CString::from_raw(v); }
    //println!("ice_core_destroy_cstring");
}
