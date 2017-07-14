use std;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use hyper;

type Pointer = usize;

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

    fn ice_glue_endpoint_handler(id: i32, req: Pointer) -> Pointer;
    pub fn ice_glue_async_endpoint_handler(id: i32, call_info: Pointer);
}

pub struct Request {
    handle: Pointer
}

impl Request {
    pub fn new() -> Request {
        Request {
            handle: unsafe { ice_glue_create_request() }
        }
    }

    pub unsafe fn from_raw(handle: Pointer) -> Request {
        Request {
            handle: handle
        }
    }

    pub fn into_raw(mut self) -> Pointer {
        let handle = self.handle;
        self.handle = 0;
        handle
    }

    pub unsafe fn get_raw(&self) -> Pointer {
        self.handle
    }

    pub fn set_remote_addr(&mut self, addr: &str) {
        unsafe { ice_glue_request_set_remote_addr(self.handle, CString::new(addr).unwrap().as_ptr()); }
    }

    pub fn set_method(&mut self, m: &str) {
        unsafe { ice_glue_request_set_method(self.handle, CString::new(m).unwrap().as_ptr()); }
    }

    pub fn set_uri(&mut self, uri: &str) {
        unsafe { ice_glue_request_set_uri(self.handle, CString::new(uri).unwrap().as_ptr()); }
    }

    pub fn add_header(&mut self, k: &str, v: &str) {
        unsafe { ice_glue_add_header(self.handle, CString::new(k).unwrap().as_ptr(), CString::new(v).unwrap().as_ptr()); }
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        if self.handle == 0 {
            return;
        }

        unsafe { ice_glue_destroy_request(self.handle); }
        self.handle = 0;
    }
}

pub struct Response {
    handle: Pointer
}

impl Response {
    pub fn new() -> Response {
        Response {
            handle: unsafe { ice_glue_create_response() }
        }
    }

    pub unsafe fn from_raw(handle: Pointer) -> Response {
        Response {
            handle: handle
        }
    }

    pub fn into_raw(mut self) -> Pointer {
        let handle = self.handle;
        self.handle = 0;
        handle
    }

    pub fn add_header(&mut self, k: &str, v: &str) {
        unsafe { ice_glue_add_header(self.handle, CString::new(k).unwrap().as_ptr(), CString::new(v).unwrap().as_ptr()); }
    }

    pub fn get_body(&self) -> Vec<u8> {
        let mut body_len: u32 = 0;
        let raw_body = unsafe { ice_glue_response_get_body(self.handle, &mut body_len) };

        unsafe { std::slice::from_raw_parts(raw_body, body_len as usize).to_vec() }
    }

    pub fn get_headers(&self) -> hyper::header::Headers {
        let itr = unsafe { ice_glue_create_header_iterator(self.handle) };
        let mut resp_headers = hyper::header::Headers::new();

        loop {
            unsafe {
                let key = ice_glue_header_iterator_next(self.handle, itr);
                if key.is_null() {
                    break;
                }
                let key = CStr::from_ptr(key);
                let value = ice_glue_get_header(self.handle, key.as_ptr());
                let key = key.to_str().unwrap();
                let value = CStr::from_ptr(value).to_str().unwrap();
                resp_headers.set_raw(key, value);
            }
        }

        unsafe { ice_glue_destroy_header_iterator(itr); }
        resp_headers
    }
}

impl Drop for Response {
    fn drop(&mut self) {
        if self.handle == 0 {
            return;
        }

        unsafe { ice_glue_destroy_response(self.handle); }
        self.handle = 0;
    }
}
