use std;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ops::Deref;
use std::cell::RefCell;
use hyper;
use ice_server;
use session_storage;
use glue::serialize;
use glue::common;

pub struct Request {
    pub uri: CString,
    pub remote_addr: CString,
    pub method: CString,
    pub url_params: HashMap<String, String>,
    pub headers: hyper::header::Headers,
    pub cookies: HashMap<String, CString>,
    pub custom_properties: Arc<common::CustomProperties>,
    pub body: Box<Deref<Target = RefCell<Vec<u8>>>>,
    pub context: Arc<ice_server::Context>,
    pub session: Option<Arc<Mutex<session_storage::Session>>>,
    pub cache: RequestCache
}

#[derive(Default)]
pub struct RequestCache {
    stats: Option<CString>,
    session_items: HashMap<String, CString>,
    headers: HashMap<String, CString>,
    url_params_raw: Option<Vec<u8>>,
    headers_raw: Option<Vec<u8>>,
    cookies_raw: Option<Vec<u8>>,
    session_items_raw: Option<Vec<u8>>
}

impl Request {
    pub fn into_boxed(self) -> Box<Request> {
        Box::new(self)
    }
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_stats(req: *mut Request) -> *const c_char {
    let req = &mut *req;

    req.cache.stats = Some(CString::new(req.context.stats.serialize().to_string()).unwrap());
    let ret = req.cache.stats.as_ref().unwrap().as_ptr();

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_uri(req: *mut Request) -> *const c_char {
    let req = &*req;

    let ret = req.uri.as_ptr();

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_method(req: *mut Request) -> *const c_char {
    let req = &*req;

    let ret = req.method.as_ptr();

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_remote_addr(req: *mut Request) -> *const c_char {
    let req = &*req;

    let ret = req.remote_addr.as_ptr();

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_url_params(req: *mut Request) -> *const u8 {
    let req = &mut *req;
    
    if req.cache.url_params_raw.is_none() {
        req.cache.url_params_raw = Some(
            serialize::std_map(req.url_params.iter(), req.url_params.len())
        );
    }

    req.cache.url_params_raw.as_ref().unwrap().as_ptr()
}

#[no_mangle]
pub unsafe fn ice_glue_request_set_custom_stat(req: *mut Request, k: *const c_char, v: *const c_char) {
    let req = &*req;

    let k = CStr::from_ptr(k).to_str().unwrap();
    let v = CStr::from_ptr(v).to_str().unwrap();

    req.context.stats.set_custom(k.to_string(), v.to_string());
}


#[no_mangle]
pub unsafe fn ice_glue_request_get_body(req: *mut Request, len_out: *mut u32) -> *const u8 {
    let req = &*req;
    let body = req.body.borrow();

    let ret = body.as_slice().as_ptr();
    *len_out = body.len() as u32;

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_header(req: *mut Request, k: *const c_char) -> *const c_char {
    let req = &mut *req;
    let k = CStr::from_ptr(k).to_str().unwrap();

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

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_headers(req: *mut Request) -> *const u8 {
    let req = &mut *req;

    if req.cache.headers_raw.is_none() {
        req.cache.headers_raw = Some(
            serialize::std_map(req.headers.iter().map(|v| {
                (v.name(), v.value_string())
            }), req.headers.len())
        );
    }

    req.cache.headers_raw.as_ref().unwrap().as_ptr()
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_cookie(req: *mut Request, k: *const c_char) -> *const c_char {
    let req = &*req;
    let k = CStr::from_ptr(k).to_str().unwrap();

    let ret = match req.cookies.get(k) {
        Some(ref v) => v.as_ptr(),
        None => std::ptr::null()
    };

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_cookies(req: *mut Request) -> *const u8 {
    let req = &mut *req;

    if req.cache.cookies_raw.is_none() {
        req.cache.cookies_raw = Some(
            serialize::std_map(req.cookies.iter().map(|(k, v)| {
                (k, match std::str::from_utf8(v.as_bytes()) {
                    Ok(v) => v,
                    Err(_) => ""
                })
            }), req.cookies.len())
        );
    }

    req.cache.cookies_raw.as_ref().unwrap().as_ptr()
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_session_item(req: *mut Request, k: *const c_char) -> *const c_char {
    let req = &mut *req;

    let k = CStr::from_ptr(k).to_str().unwrap();
    let ret;

    {
        let v = match req.session {
            Some(ref session) => {
                match session.lock().unwrap().data.get(k) {
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

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_get_session_items(req: *mut Request) -> *const u8 {
    let req = &mut *req;

    match req.session {
        Some(ref session) => {
            let session = session.lock().unwrap();
            req.cache.session_items_raw = Some(
                serialize::std_map(session.data.iter(), session.data.len())
            );
            req.cache.session_items_raw.as_ref().unwrap().as_ptr()
        },
        None => std::ptr::null()
    }
}

#[no_mangle]
pub unsafe fn ice_glue_request_set_session_item(req: *mut Request, k: *const c_char, value: *const c_char) {
    let req = &*req;
    let k = CStr::from_ptr(k).to_str().unwrap();

    match req.session {
        Some(ref session) => {
            match value.is_null() {
                true => {
                    session.lock().unwrap().data.remove(k);
                },
                false => {
                    let value = CStr::from_ptr(value).to_str().unwrap();
                    session.lock().unwrap().data.insert(k.to_string(), value.to_string());
                }
            }
        },
        None => {}
    }
}

#[no_mangle]
pub unsafe fn ice_glue_request_render_template_to_owned(req: *mut Request, name: *const c_char, data: *const c_char) -> *mut c_char {
    let req = &*req;

    let ret = match req.context.templates.render_json(
        CStr::from_ptr(name).to_str().unwrap(),
        CStr::from_ptr(data).to_str().unwrap()
    ) {
        Some(v) => CString::new(v).unwrap().into_raw(),
        None => std::ptr::null_mut()
    };

    ret
}

#[no_mangle]
pub unsafe fn ice_glue_request_borrow_context(req: *mut Request) -> *const ice_server::Context {
    let req = &*req;
    &*req.context
}

#[no_mangle]
pub unsafe fn ice_glue_request_borrow_custom_properties(req: *mut Request) -> *const common::CustomProperties {
    let req = &*req;
    &*req.custom_properties
}
