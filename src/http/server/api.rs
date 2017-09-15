use std;
use std::os::raw::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::net::SocketAddr;
use futures::Future;
use hyper;
use http::server::config::HttpServerConfig;
use http::server::{HttpServer, HttpServerExecutionContext};
use http::server::router::RouteInfo;
use http::server::endpoint_context::EndpointContext;

#[no_mangle]
pub extern "C" fn ice_http_server_config_create() -> *mut HttpServerConfig {
    Box::into_raw(Box::new(HttpServerConfig::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_config_set_listen_addr(
    config: &mut HttpServerConfig,
    addr: *const c_char
) {
    let addr = CStr::from_ptr(addr).to_str().unwrap();
    let addr: SocketAddr = addr.parse().unwrap();

    config.listen_addr = Some(addr);
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_config_set_num_executors(
    config: &mut HttpServerConfig,
    n: u32
) {
    config.num_executors = n as usize;
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_config_destroy(config: *mut HttpServerConfig) {
    Box::from_raw(config);
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_create(config: *mut HttpServerConfig) -> *mut HttpServer {
    Box::into_raw(Box::new(HttpServer::new(
        *Box::from_raw(config)
    )))
}

#[no_mangle]
pub extern "C" fn ice_http_server_start(
    server: &HttpServer
) -> *mut HttpServerExecutionContext {
    match server.start() {
        Some(v) => Box::into_raw(Box::new(v)),
        None => std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_route_create(
    path: *const c_char,
    cb: extern fn (*mut EndpointContext, *const hyper::Request, *const c_void),
    call_with: *const c_void
) -> *mut RouteInfo {
    let path = CStr::from_ptr(path).to_str().unwrap();
    let call_with = call_with as usize;

    let callback = move |req: hyper::Request| -> Box<Future<Item = hyper::Response, Error = hyper::Error>> {
        let call_with = call_with as *const c_void;
        let (ctx, ret) = EndpointContext::new_pair(Box::new(req));
        let ctx = Box::new(ctx);
        let req = ctx.get_request() as *const hyper::Request;

        cb(Box::into_raw(ctx), req, call_with);

        ret
    };

    Box::into_raw(Box::new(RouteInfo::new(path, Box::new(callback))))
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_route_destroy(
    rt: *mut RouteInfo
) {
    Box::from_raw(rt);
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_add_route(
    server: &HttpServer,
    rt: *mut RouteInfo
) {
    let rt = Box::from_raw(rt);

    let mut table = server.get_routing_table_mut();
    table.add_route(*rt);
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_server_endpoint_context_end_with_response(
    ctx: *mut EndpointContext,
    resp: *mut hyper::Response
) -> bool {
    let ctx = Box::from_raw(ctx);
    let resp = Box::from_raw(resp);

    ctx.end(*resp)
}
