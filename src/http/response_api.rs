use std;
use std::ffi::CStr;
use std::os::raw::c_char;
use hyper;

#[no_mangle]
pub extern "C" fn ice_http_response_create() -> *mut hyper::Response {
    Box::into_raw(Box::new(hyper::Response::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_response_destroy(resp: *mut hyper::Response) {
    Box::from_raw(resp);
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_response_set_status(resp: &mut hyper::Response, status: u16) {
    resp.set_status(match hyper::StatusCode::try_from(status) {
        Ok(v) => v,
        Err(_) => return
    });
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_response_set_header(
    resp: &mut hyper::Response,
    k: *const c_char,
    v: *const c_char
) {
    let k = CStr::from_ptr(k).to_str().unwrap();
    let v = CStr::from_ptr(v).to_str().unwrap();

    resp.headers_mut().set_raw(k.to_string(), v.to_string());
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_response_append_header(
    resp: &mut hyper::Response,
    k: *const c_char,
    v: *const c_char
) {
    let k = CStr::from_ptr(k).to_str().unwrap();
    let v = CStr::from_ptr(v).to_str().unwrap();

    resp.headers_mut().append_raw(k.to_string(), v.to_string());
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_response_set_body(
    resp: &mut hyper::Response,
    data: *const u8,
    len: u32
) {
    let data = std::slice::from_raw_parts(data, len as usize);
    resp.headers_mut().set(hyper::header::ContentLength(len as u64));
    resp.set_body(data.to_vec());
}
