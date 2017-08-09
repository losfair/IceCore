use std;
use std::sync::Mutex;
use std::ffi::{CStr, CString};
use std::collections::HashMap;
use std::os::raw::c_char;

#[derive(Default)]
pub struct CustomProperties {
    pub fields: Mutex<HashMap<String, CString>>
}

#[no_mangle]
pub unsafe fn ice_glue_custom_properties_set(cp: *const CustomProperties, k: *const c_char, v: *const c_char) {
    let cp = &*cp;

    cp.fields.lock().unwrap().insert(
        CStr::from_ptr(k).to_str().unwrap().to_string(),
        CString::new(CStr::from_ptr(v).to_str().unwrap()).unwrap()
    );
}

#[no_mangle]
pub unsafe fn ice_glue_custom_properties_get(cp: *const CustomProperties, k: *const c_char) -> *const c_char {
    let cp = &*cp;

    match cp.fields.lock().unwrap().get(CStr::from_ptr(k).to_str().unwrap()) {
        Some(v) => v.as_ptr(),
        None => std::ptr::null()
    }
}
