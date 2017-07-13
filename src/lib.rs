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
pub fn ice_server_listen(handle: ServerHandle, addr: *const c_char) {
    let handle = unsafe { Arc::from_raw(handle) };

    {
        let server = handle.lock().unwrap();
        server.listen(unsafe { CStr::from_ptr(addr) }.to_str().unwrap());
    }

    Arc::into_raw(handle);
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
