use std;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use hyper;
use glue;
use ice_server;
use streaming;

#[derive(Debug)]
pub struct Response {
    pub body: Vec<u8>,
    pub file: Option<String>,
    pub status: u16,
    pub headers: hyper::header::Headers,
    pub cookies: HashMap<String, String>,
    pub stream_rx: Option<streaming::ChunkReceiver>
}

impl Response {
    pub fn new() -> Response {
        Response {
            body: Vec::new(),
            file: None,
            status: 200,
            headers: hyper::header::Headers::new(),
            cookies: HashMap::new(),
            stream_rx: None
        }
    }

    pub fn into_boxed(self) -> Box<Response> {
        Box::new(self)
    }

    pub fn add_header(&mut self, k: &str, v: &str) {
        self.headers.set_raw(glue::header::transform_name(k), v.to_string());
    }

    pub fn set_cookie(&mut self, k: &str, v: &str) {
        self.cookies.insert(k.to_string(), v.to_string());
    }

    pub fn set_body(&mut self, data: &[u8]) {
        self.body = data.to_vec();
    }

    pub fn set_file(&mut self, path: &str) {
        self.file = Some(path.to_string());
    }

    pub fn set_status(&mut self, status: u16) {
        self.status = status;
    }

    pub fn stream(&mut self, ctx: &ice_server::Context) -> streaming::StreamProvider {
        if self.stream_rx.is_some() {
            panic!("Attempting to enable streaming for a response that has already enabled it");
        }

        let (provider, rx) = streaming::StreamProvider::new(&ctx.ev_loop_remote);
        self.stream_rx = Some(rx);

        provider
    }
}

#[no_mangle]
pub fn ice_glue_create_response() -> *mut Response {
    Box::into_raw(Response::new().into_boxed())
}

#[no_mangle]
pub unsafe fn ice_glue_response_add_header(resp: *mut Response, k: *const c_char, v: *const c_char) {
    let resp = &mut *resp;

    resp.add_header(CStr::from_ptr(k).to_str().unwrap(), CStr::from_ptr(v).to_str().unwrap());
}

#[no_mangle]
pub unsafe fn ice_glue_response_set_cookie(resp: *mut Response, k: *const c_char, v: *const c_char) {
    let resp = &mut *resp;

    resp.set_cookie(CStr::from_ptr(k).to_str().unwrap(), CStr::from_ptr(v).to_str().unwrap());
}

#[no_mangle]
pub unsafe fn ice_glue_response_set_body(resp: *mut Response, data: *const u8, len: u32) {
    let resp = &mut *resp;

    if data.is_null() || len == 0 {
        resp.set_body(&[]);
    } else {
        resp.set_body(std::slice::from_raw_parts(data, len as usize));
    }
}

#[no_mangle]
pub unsafe fn ice_glue_response_set_file(resp: *mut Response, path: *const c_char) {
    let resp = &mut *resp;

    resp.set_file(CStr::from_ptr(path).to_str().unwrap());
}

#[no_mangle]
pub unsafe fn ice_glue_response_set_status(resp: *mut Response, status: u16) {
    let resp = &mut *resp;

    resp.set_status(status);
}

#[no_mangle]
pub unsafe fn ice_glue_response_consume_rendered_template(resp: *mut Response, content: *mut c_char) {
    let resp = &mut *resp;
    let content = CString::from_raw(content);

    resp.set_body(content.as_bytes());
}

#[no_mangle]
pub unsafe fn ice_glue_response_stream(resp: *mut Response, ctx: *const ice_server::Context) -> *mut streaming::StreamProvider {
    let resp = &mut *resp;
    let ctx = &*ctx;

    Box::into_raw(resp.stream(ctx).into_boxed())
}
