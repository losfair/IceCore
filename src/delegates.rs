use std;
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

    fn ice_glue_create_response() -> Pointer;
    fn ice_glue_destroy_response(resp: Pointer);
    fn ice_glue_response_get_body(resp: Pointer, k: *mut u32) -> *const u8;

    fn ice_glue_get_header(t: Pointer, k: *const c_char) -> *const c_char;
    fn ice_glue_add_header(t: Pointer, k: *const c_char, v: *const c_char);
    fn ice_glue_create_header_iterator(t: Pointer) -> Pointer;
    fn ice_glue_destroy_header_iterator(itr_p: Pointer);
    fn ice_glue_header_iterator_next(t: Pointer, itr_p: Pointer) -> *const c_char;

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

    for hdr in req.headers().iter() {
        let name = CString::new(hdr.name()).unwrap().into_raw();
        let value = CString::new(hdr.value_string()).unwrap().into_raw();

        ice_glue_add_header(raw_req, name, value);

        CString::from_raw(name);
        CString::from_raw(value);
    }

    let mut resp_headers = hyper::header::Headers::new();

    let server_id = CString::new("").unwrap().into_raw();
    let raw_resp = ice_glue_endpoint_handler(server_id, raw_req);
    CString::from_raw(server_id);

    let resp_hdr = ice_glue_create_header_iterator(raw_resp);

    loop {
        let key = ice_glue_header_iterator_next(raw_resp, resp_hdr);
        if key.is_null() {
            break;
        }
        let key = CStr::from_ptr(key);
        let value = ice_glue_get_header(raw_resp, key.as_ptr());
        let key = key.to_str().unwrap();
        let value = CStr::from_ptr(value).to_str().unwrap();
        resp_headers.set_raw(key, value);
    }

    ice_glue_destroy_header_iterator(resp_hdr);

    let mut resp_body_len: u32 = 0;
    let resp_body = ice_glue_response_get_body(raw_resp, &mut resp_body_len);
    let resp_body = std::slice::from_raw_parts(resp_body, resp_body_len as usize);

    ice_glue_destroy_response(raw_resp);
    ice_glue_destroy_request(raw_req);

    CString::from_raw(remote_addr);
    CString::from_raw(method);
    CString::from_raw(uri);

    Response::new().with_headers(resp_headers).with_body(resp_body)
}
