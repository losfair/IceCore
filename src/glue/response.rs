use std;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use hyper;
use glue;

pub struct Response {
    pub body: Vec<u8>,
    pub file: Option<String>,
    pub status: u16,
    pub headers: hyper::header::Headers,
    pub cookies: HashMap<String, String>
}

impl Response {
    pub fn new() -> Response {
        Response {
            body: Vec::new(),
            file: None,
            status: 200,
            headers: hyper::header::Headers::new(),
            cookies: HashMap::new()
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
}

#[no_mangle]
pub fn ice_glue_create_response() -> *mut Response {
    Box::into_raw(Response::new().into_boxed())
}

#[no_mangle]
pub fn ice_glue_response_add_header(resp: *mut Response, k: *const c_char, v: *const c_char) {
    let mut resp = unsafe { Box::from_raw(resp) };

    resp.add_header(unsafe { CStr::from_ptr(k) }.to_str().unwrap(), unsafe { CStr::from_ptr(v) }.to_str().unwrap());

    Box::into_raw(resp);
}

#[no_mangle]
pub fn ice_glue_response_set_cookie(resp: *mut Response, k: *const c_char, v: *const c_char) {
    let mut resp = unsafe { Box::from_raw(resp) };

    resp.set_cookie(unsafe { CStr::from_ptr(k) }.to_str().unwrap(), unsafe { CStr::from_ptr(v) }.to_str().unwrap());

    Box::into_raw(resp);
}

#[no_mangle]
pub fn ice_glue_response_set_body(resp: *mut Response, data: *const u8, len: u32) {
    let mut resp = unsafe { Box::from_raw(resp) };

    if data.is_null() || len == 0 {
        resp.set_body(&[]);
    } else {
        resp.set_body(unsafe { std::slice::from_raw_parts(data, len as usize) });
    }

    Box::into_raw(resp);
}

#[no_mangle]
pub fn ice_glue_response_set_file(resp: *mut Response, path: *const c_char) {
    let mut resp = unsafe { Box::from_raw(resp) };

    resp.set_file(unsafe { CStr::from_ptr(path) }.to_str().unwrap());

    Box::into_raw(resp);
}

#[no_mangle]
pub fn ice_glue_response_set_status(resp: *mut Response, status: u16) {
    let mut resp = unsafe { Box::from_raw(resp) };

    resp.set_status(status);

    Box::into_raw(resp);
}

#[no_mangle]
pub fn ice_glue_response_consume_rendered_template(resp: *mut Response, content: *mut c_char) {
    let mut resp = unsafe { Box::from_raw(resp) };
    let content = unsafe { CString::from_raw(content) };

    resp.set_body(content.as_bytes());

    Box::into_raw(resp);
}
