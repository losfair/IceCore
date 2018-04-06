#![feature(fnbox)]

use std::boxed::FnBox;

extern "C" {
    fn __ice_drop_task(task_id: i32);
    fn __ice_request_timeout(
        timeout: i64,
        cb: extern "C" fn (user_data: i32) -> i32,
        user_data: i32
    );
    fn __ice_request_instant(
        cb: extern "C" fn (user_data: i32) -> i32,
        user_data: i32
    );
    fn __ice_log(
        str_base: *const u8,
        str_len: usize
    );
}

pub fn write_log(s: &str) {
    let s = s.as_bytes();
    unsafe {
        __ice_log(
            &s[0],
            s.len()
        );
    }
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::write_log(&format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => ($crate::write_log(&format!($fmt, $($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    ($fmt:expr) => ($crate::write_log(&format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => ($crate::write_log(&format!($fmt, $($arg)*)));
}

#[macro_export]
macro_rules! app_init {
    ($body:block) => {
        #[no_mangle]
        pub extern "C" fn __app_init() -> i32 {
            $body
        }
    }
}

#[no_mangle]
pub extern "C" fn __app_invoke0(
    target: extern "C" fn () -> i32
) -> i32 {
    target()
}

#[no_mangle]
pub extern "C" fn __app_invoke1(
    target: extern "C" fn (i32) -> i32,
    arg1: i32
) -> i32 {
    target(arg1)
}

#[no_mangle]
pub extern "C" fn __app_invoke2(
    target: extern "C" fn (i32, i32) -> i32,
    arg1: i32,
    arg2: i32
) -> i32 {
    target(arg1, arg2)
}

#[no_mangle]
pub extern "C" fn __app_invoke3(
    target: extern "C" fn (i32, i32, i32) -> i32,
    arg1: i32,
    arg2: i32,
    arg3: i32
) -> i32 {
    target(arg1, arg2, arg3)
}

#[no_mangle]
pub extern "C" fn __app_invoke4(
    target: extern "C" fn (i32, i32, i32, i32) -> i32,
    arg1: i32,
    arg2: i32,
    arg3: i32,
    arg4: i32
) -> i32 {
    target(arg1, arg2, arg3, arg4)
}

pub fn set_timeout<T: FnOnce()>(ms: i64, cb: T) {
    extern "C" fn raw_cb(addr: i32) -> i32 {
        let f: Box<Box<FnBox()>> = unsafe {
            Box::from_raw(addr as *mut Box<FnBox()>)
        };
        (*f)();
        0
    }
    let f: Box<Box<FnBox()>> = Box::new(Box::new(cb));
    let f = Box::into_raw(f);
    unsafe {
        __ice_request_timeout(
            ms,
            raw_cb,
            f as _
        );
    }
}

pub fn schedule<T: FnOnce()>(cb: T) {
    extern "C" fn raw_cb(addr: i32) -> i32 {
        let f: Box<Box<FnBox()>> = unsafe {
            Box::from_raw(addr as *mut Box<FnBox()>)
        };
        (*f)();
        0
    }
    let f: Box<Box<FnBox()>> = Box::new(Box::new(cb));
    let f = Box::into_raw(f);
    unsafe {
        __ice_request_instant(
            raw_cb,
            f as _
        );
    }
}
