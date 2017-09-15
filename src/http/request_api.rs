use std::ffi::CString;
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
