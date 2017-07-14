extern crate hyper;
extern crate futures;

mod ice_server;
mod delegates;
mod router;
mod glue;

use std::sync::{Arc, Mutex};
use std::ffi::CStr;
use std::os::raw::c_char;
use ice_server::IceServer;
use delegates::ServerHandle;

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
pub fn ice_server_router_add_endpoint(handle: ServerHandle, p: *const c_char) -> i32 {
    let handle = unsafe { Arc::from_raw(handle) };
    let mut id: i32;

    {
        let mut server = handle.lock().unwrap();
        let mut router = server.context.router.write().unwrap();
        id = router.add_endpoint(unsafe { CStr::from_ptr(p) }.to_str().unwrap());
    }

    Arc::into_raw(handle);

    id
}

#[no_mangle]
pub fn ice_core_fire_callback(call_info: *mut delegates::CallInfo, resp: delegates::Pointer) {
    let call_info = unsafe { Box::from_raw(call_info) };

    call_info.tx.complete(resp);
}

#[no_mangle]
pub fn ice_core_borrow_request_from_call_info(call_info: *mut delegates::CallInfo) -> delegates::Pointer {
    let call_info = unsafe { Box::from_raw(call_info) };

    let raw_req = unsafe { call_info.req.get_raw() };

    Box::into_raw(call_info);

    raw_req
}
