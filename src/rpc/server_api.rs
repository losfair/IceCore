use std::ffi::CStr;
use std::os::raw::c_char;
use super::{RpcServer, RpcServerConfig};
use rpc::service::RpcService;

#[no_mangle]
pub extern "C" fn ice_rpc_server_config_create() -> *mut RpcServerConfig {
    Box::into_raw(Box::new(RpcServerConfig::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_server_config_destroy(config: *mut RpcServerConfig) {
    Box::from_raw(config);
}

#[no_mangle]
pub extern "C" fn ice_rpc_server_create(config: *mut RpcServerConfig) -> *mut RpcServer {
    Box::into_raw(Box::new(RpcServer::new(
        * unsafe {
            Box::from_raw(config)
        }
    )))
}

#[no_mangle]
pub extern "C" fn ice_rpc_server_start(server: &RpcServer, addr: *const c_char) {
    let addr = unsafe {
        CStr::from_ptr(addr).to_str().unwrap()
    };

    let svc = RpcService::new(server);
    svc.start(addr);
}
