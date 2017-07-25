use std;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use hyper;
use glue::common;
use ice_server;
use session_storage;

pub struct Request {
    pub uri: String,
    pub remote_addr: String,
    pub method: String,
    pub headers: hyper::header::Headers,
    pub cookies: HashMap<String, CString>,
    pub body: Vec<u8>,
    pub context: Arc<ice_server::Context>,
    pub session: Option<Arc<RwLock<session_storage::Session>>>,
    cache: RequestCache
}

struct RequestCache {
    stats: Option<CString>,
    session_items: HashMap<String, CString>,
    headers: HashMap<String, CString>
}

impl Request {
    pub fn into_boxed(self) -> Box<Request> {
        Box::new(self)
    }
}

#[no_mangle]
pub fn ice_glue_request_get_stats(req: *mut Request) -> *const c_char {
    let mut req = unsafe { Box::from_raw(req) };

    req.cache.stats = Some(CString::new(req.context.stats.serialize().to_string()).unwrap());
    let ret = req.cache.stats.as_ref().unwrap().as_ptr();

    Box::into_raw(req);
    ret
}


#[no_mangle]
pub fn ice_glue_request_set_custom_stat(req: *mut Request, k: *const c_char, v: *const c_char) {
    let mut req = unsafe { Box::from_raw(req) };
    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap();
    let v = unsafe { CStr::from_ptr(v) }.to_str().unwrap();

    req.context.stats.set_custom(k.to_string(), v.to_string());

    Box::into_raw(req);
}


#[no_mangle]
pub fn ice_glue_request_get_body(req: *mut Request, len_out: *mut u32) -> *const u8 {
    let mut req = unsafe { Box::from_raw(req) };

    let ret = req.body.as_slice().as_ptr();
    unsafe { *len_out = req.body.len() as u32; }

    Box::into_raw(req);
    ret
}

#[no_mangle]
pub fn ice_glue_request_get_header(req: *mut Request, k: *const c_char) -> *const c_char {
    let mut req = unsafe { Box::from_raw(req) };
    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap();

    let ret = match req.headers.get_raw(k) {
        Some(v) => match v.one() {
            Some(v) => match std::str::from_utf8(v) {
                Ok(v) => Some(CString::new(v).unwrap()),
                Err(_) => None
            },
            None => None
        },
        None => None
    };
    let ret = match ret {
        Some(v) => {
            req.cache.headers.insert(k.to_string(), v);
            req.cache.headers.get(k).as_ref().unwrap().as_ptr()
        },
        None => std::ptr::null()
    };

    Box::into_raw(req);
    ret
}

#[no_mangle]
pub fn ice_glue_request_get_cookie(req: *mut Request, k: *const c_char) -> *const c_char {
    let mut req = unsafe { Box::from_raw(req) };
    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap();

    let ret = match req.cookies.get(k) {
        Some(ref v) => v.as_ptr(),
        None => std::ptr::null()
    };

    Box::into_raw(req);
    ret
}

#[no_mangle]
pub fn ice_glue_request_get_session_item(req: *mut Request, k: *const c_char) -> *const c_char {
    let mut req = unsafe { Box::from_raw(req) };
    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap();
    let ret;

    {
        let v = match req.session {
            Some(ref session) => {
                match session.read().unwrap().data.get(k) {
                    Some(v) => {
                        Some(CString::new(v.as_str()).unwrap())
                    },
                    None => None
                }
            },
            None => None
        };

        let mut session_items = &mut req.cache.session_items;
        ret = match v {
            Some(v) => {
                session_items.insert(k.to_string(), v);
                session_items.get(k).as_ref().unwrap().as_ptr()
            },
            None => std::ptr::null()
        };
    }

    Box::into_raw(req);
    ret
}

#[no_mangle]
pub fn ice_glue_request_set_session_item(req: *mut Request, k: *const c_char, value: *const c_char) {
    let mut req = unsafe { Box::from_raw(req) };
    let k = unsafe { CStr::from_ptr(k) }.to_str().unwrap();

    match req.session {
        Some(ref session) => {
            match value.is_null() {
                true => {
                    session.write().unwrap().data.remove(k);
                },
                false => {
                    let value = unsafe { CStr::from_ptr(value) }.to_str().unwrap();
                    session.write().unwrap().data.insert(k.to_string(), value.to_string());
                }
            }
        },
        None => {}
    }

    Box::into_raw(req);
}


// Will be deprecated.
#[no_mangle]
pub fn ice_glue_request_create_header_iterator(req: *mut Request) -> *mut common::HeaderIterator {
    let mut req = unsafe { Box::from_raw(req) };

    let headers = req.headers.iter().map(|hdr| {
        (CString::new(hdr.name()).unwrap(), CString::new(hdr.value_string()).unwrap())
    }).collect();
    let itr = common::HeaderIterator {
        headers: headers,
        pos: 0
    };

    Box::into_raw(req);
    Box::into_raw(Box::new(itr))
}

#[no_mangle]
pub fn ice_glue_request_header_iterator_next(req: *mut Request, itr: *mut common::HeaderIterator) -> *const c_char {
    let mut itr = unsafe { Box::from_raw(itr) };

    let ret = if itr.pos >= itr.headers.len() {
        std::ptr::null()
    } else {
        itr.headers[itr.pos].0.as_ptr()
    };

    Box::into_raw(itr);
    ret
}

