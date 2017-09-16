use std;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::rc::Rc;
use std::cell::RefCell;
use futures::Future;
use futures::Stream;
use hyper;
use executor;

#[no_mangle]
pub unsafe extern "C" fn ice_http_request_destroy(
    req: *mut hyper::Request
) {
    Box::from_raw(req);
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_request_get_uri_to_owned(
    req: &hyper::Request
) -> *mut c_char {
    CString::new(format!("{}", req.uri())).unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_request_get_method_to_owned(
    req: &hyper::Request
) -> *mut c_char {
    CString::new(format!("{}", req.method())).unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_request_get_remote_addr_to_owned(
    req: &hyper::Request
) -> *mut c_char {
    CString::new(match req.remote_addr() {
        Some(v) => format!("{}", v),
        None => "".to_string()
    }).unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_request_get_header_to_owned(
    req: &hyper::Request,
    key: *const c_char
) -> *mut c_char {
    let key = CStr::from_ptr(key).to_str().unwrap();

    match req.headers().get_raw(key) {
        Some(v) => match v.one() {
            Some(v) => match CString::new(v) {
                Ok(v) => v.into_raw(),
                Err(_) => std::ptr::null_mut()
            },
            None => std::ptr::null_mut()
        },
        None => std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_request_iter_headers(
    req: &hyper::Request,
    cb: extern fn (*const c_char, *const c_char, *const c_void),
    call_with: *const c_void
)  {
    for h in req.headers().iter() {
        let name = CString::new(h.name()).unwrap();
        let value = CString::new(h.value_string()).unwrap();

        cb(name.as_ptr(), value.as_ptr(), call_with);
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_request_take_and_read_body(
    req: *mut hyper::Request,
    cb_on_data: extern fn (*const u8, u32, *const c_void) -> bool,
    cb_on_end: extern fn (bool, *const c_void),
    call_with: *const c_void
) {
    let req = Box::from_raw(req);
    let body = req.body();
    let call_with = call_with as usize;

    executor::get_event_loop().spawn(move |_| {
        let call_with = call_with as *const c_void;

        body.for_each(move |chunk| {
            let should_continue = cb_on_data(chunk.as_ptr(), chunk.len() as u32, call_with);
            if should_continue {
                Ok(())
            } else {
                Err(hyper::Error::Incomplete)
            }
        }).then(move |result| {
            let ok = match result {
                Ok(_) => true,
                Err(_) => false
            };
            cb_on_end(ok, call_with);
            Ok(())
        })
    });
}
