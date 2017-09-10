use std::os::raw::c_char;
use std::ffi::CString;

pub static VERSION: &'static str = env!("CARGO_PKG_VERSION");
lazy_static! {
    static ref VERSION_C: CString = {
        CString::new(VERSION).unwrap()
    };
}

#[no_mangle]
pub extern "C" fn ice_metadata_get_version() -> *const c_char {
    VERSION_C.as_ptr()
}
