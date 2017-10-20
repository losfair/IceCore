use std::ptr;
use std::mem;
use std::cell::RefCell;
use std::os::raw::c_char;
use std::ffi::CString;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None);
}

#[no_mangle]
pub extern "C" fn ice_get_last_error() -> *const c_char {
    let mut ret: *const c_char = ptr::null();

    LAST_ERROR.with(|v| {
        let v = v.borrow();
        ret = match *v {
            Some(ref v) => v.as_ptr(),
            None => ptr::null()
        }
    });

    ret
}

pub unsafe fn get_last_error_unsafe() -> Option<&'static str> {
    let mut ret: Option<&'static str> = None;

    LAST_ERROR.with(|v| {
        let v = v.borrow();
        ret = match *v {
            Some(ref v) => Some(mem::transmute(v.to_str().unwrap())),
            None => None
        }
    });

    ret
}

pub fn get_last_error() -> Option<String> {
    match unsafe { get_last_error_unsafe() } {
        Some(v) => Some(v.to_string()),
        None => None
    }
}

pub fn set_last_error<T: AsRef<str>>(e: T) {
    LAST_ERROR.with(|v| {
        *v.borrow_mut() = Some(CString::new(e.as_ref()).unwrap());
    });
}

pub fn clear_last_error() {
    LAST_ERROR.with(|v| {
        *v.borrow_mut() = None;
    });
}
