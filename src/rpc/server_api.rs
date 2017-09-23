use std::ffi::CStr;
use std::os::raw::{c_void, c_char};
use futures::Future;
use super::{RpcServer, RpcServerConfig};
use rpc::service::RpcService;
use rpc::call_context::CallContext;

#[no_mangle]
pub extern "C" fn ice_rpc_server_config_create() -> *mut RpcServerConfig {
    Box::into_raw(Box::new(RpcServerConfig::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_server_config_destroy(config: *mut RpcServerConfig) {
    Box::from_raw(config);
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_server_config_add_method(
    config: &mut RpcServerConfig,
    name: *const c_char,
    f: extern "C" fn (ctx: *mut CallContext, call_with: *const c_void),
    call_with: *const c_void
) {
    let call_with = call_with as usize;
    let name = CStr::from_ptr(name).to_str().unwrap();

    config.methods.insert(name.to_string(), Box::new(move |params| {
        let call_with = call_with as *const c_void;
        let (ctx, rx) = CallContext::new(params);

        let ctx = Box::new(ctx);
        f(Box::into_raw(ctx), call_with);

        Box::new(rx.map_err(|_| ()))
    }));
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
