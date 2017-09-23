use std;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use rpc::param::Param;

#[no_mangle]
pub extern "C" fn ice_rpc_param_build_i32(v: i32) -> *mut Param {
    Box::into_raw(Box::new(Param::Integer(v)))
}

#[no_mangle]
pub extern "C" fn ice_rpc_param_build_f64(v: f64) -> *mut Param {
    Box::into_raw(Box::new(Param::Float(v)))
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_build_string(v: *const c_char) -> *mut Param {
    let v = CStr::from_ptr(v).to_str().unwrap();
    Box::into_raw(Box::new(Param::String(v.to_string())))
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_build_error(from: *mut Param) -> *mut Param {
    let from = Box::from_raw(from);
    Box::into_raw(Box::new(Param::Error(from)))
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_build_bool(v: bool) -> *mut Param {
    Box::into_raw(Box::new(Param::Boolean(v)))
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_build_null() -> *mut Param {
    Box::into_raw(Box::new(Param::Null))
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_get_i32(p: &Param) -> i32 {
    match p {
        &Param::Integer(v) => v,
        _ => 0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_get_f64(p: &Param) -> f64 {
    match p {
        &Param::Float(v) => v,
        _ => 0.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_get_string_to_owned(p: &Param) -> *mut c_char {
    match p {
        &Param::String(ref v) => CString::new(v.as_str()).unwrap().into_raw(),
        _ => std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_get_bool(p: &Param) -> bool {
    match p {
        &Param::Boolean(v) => v,
        _ => false
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_is_null(p: &Param) -> bool {
    match p {
        &Param::Null => true,
        _ => false
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_param_destroy(p: *mut Param) {
    Box::from_raw(p);
}
