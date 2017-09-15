use std;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use hyper;

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
