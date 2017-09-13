use std;
use http::server::config::HttpServerConfig;
use http::server::{HttpServer, HttpServerExecutionContext};

#[no_mangle]
pub extern "C" fn ice_http_server_config_create() -> *mut HttpServerConfig {
    Box::into_raw(Box::new(HttpServerConfig::new()))
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
