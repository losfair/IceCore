use std::sync::{Arc, Mutex};
use ice_server::IceServer;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};

use hyper;
use hyper::server::{Request, Response};

use ice_server;

pub type ServerHandle = Arc<Mutex<IceServer>>;
pub type Pointer = usize;

#[no_mangle]
extern {
    fn ice_glue_create_request() -> Pointer;
    fn ice_glue_destroy_request(req: Pointer);
    fn ice_glue_request_set_remote_addr(req: Pointer, addr: *const c_char);
    fn ice_glue_request_set_method(req: Pointer, m: *const c_char);
    fn ice_glue_request_set_uri(req: Pointer, uri: *const c_char);
    fn ice_glue_request_add_header(req: Pointer, k: *const c_char, v: *const c_char);

    fn ice_glue_create_response() -> Pointer;
    fn ice_glue_destroy_response(resp: Pointer);
    fn ice_glue_response_get_header(resp: Pointer, k: *const c_char) -> *const c_char;
    fn ice_glue_response_get_body(resp: Pointer, k: *mut u32) -> *const u8;
    fn ice_glue_response_create_header_iterator(resp: Pointer) -> Pointer;
    fn ice_glue_response_header_iterator_next(resp: Pointer, itr_p: Pointer) -> *const c_char;

    fn ice_glue_endpoint_handler(server_id: *const c_char, req: Pointer) -> Pointer;
}

pub unsafe fn handle_request(ctx: &ice_server::Context, req: &Request) -> Response {
    let raw_req = ice_glue_create_request();

    let remote_addr = CString::new(format!("{}", req.remote_addr().unwrap())).unwrap().into_raw();
    let method = CString::new(format!("{}", req.method())).unwrap().into_raw();
    let uri = CString::new(format!("{}", req.uri())).unwrap().into_raw();

    ice_glue_request_set_remote_addr(raw_req, remote_addr);
    ice_glue_request_set_method(raw_req, method);
    ice_glue_request_set_uri(raw_req, uri);

    CString::from_raw(remote_addr);
    CString::from_raw(method);
    CString::from_raw(uri);

    let mut headers = hyper::header::Headers::new();

    let server_id = CString::new("").unwrap().into_raw();
    let raw_resp = ice_glue_endpoint_handler(server_id, raw_req);
    CString::from_raw(server_id);

    let itr = ice_glue_response_create_header_iterator(raw_resp);

    loop {
        let key = ice_glue_response_header_iterator_next(raw_resp, itr);
        if key.is_null() {
            break;
        }
        let key = CStr::from_ptr(key);
        let value = ice_glue_response_get_header(raw_resp, key.as_ptr());
        let key = key.to_str().unwrap();
        let value = CStr::from_ptr(value).to_str().unwrap();
        headers.set_raw(key, value);
    }

    ice_glue_destroy_response(raw_resp);
    ice_glue_destroy_request(raw_req);

    Response::new().with_headers(headers).with_body("Nothing")
}
