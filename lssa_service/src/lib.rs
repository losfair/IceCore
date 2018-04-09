#![feature(fnbox)]

pub extern crate futures;

pub mod executor;
pub mod utils;
pub mod error;

use std::boxed::FnBox;
use std::rc::Rc;
use std::ops::Deref;

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
    fn __ice_current_time_ms() -> i64;
    fn __ice_tcp_listen(
        addr_base: *const u8,
        addr_len: usize,
        cb: extern "C" fn (stream_tid: i32, user_data: i32) -> i32,
        user_data: i32
    ) -> i32;
    fn __ice_tcp_write(
        stream_tid: i32,
        data_base: *const u8,
        data_len: usize,
        cb: extern "C" fn (len: i32, user_data: i32) -> i32,
        user_data: i32
    ) -> i32;
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

pub fn time() -> i64 {
    unsafe {
        __ice_current_time_ms()
    }
}

#[derive(Clone)]
pub struct TcpStream {
    inner: Rc<TcpStreamImpl>
}

unsafe impl Send for TcpStream {}
unsafe impl Sync for TcpStream {}

impl Deref for TcpStream {
    type Target = TcpStreamImpl;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

pub struct TcpStreamImpl {
    handle: i32
}

impl Drop for TcpStreamImpl {
    fn drop(&mut self) {
        unsafe {
            __ice_drop_task(self.handle);
        }
    }
}

impl TcpStreamImpl {
    pub fn write<F: FnOnce(i32)>(&self, data: &[u8], cb: F) -> i32 {
        extern "C" fn raw_cb(len: i32, user_data: i32) -> i32 {
            let cb: Box<Box<FnBox(i32)>> = unsafe { Box::from_raw(
                user_data as *mut Box<FnBox(i32)>
            ) };
            cb(len);
            0
        }
        let cb: Box<Box<FnBox(i32)>> = Box::new(Box::new(cb));

        unsafe {
            __ice_tcp_write(
                self.handle,
                &data[0],
                data.len(),
                raw_cb,
                Box::into_raw(cb) as _
            )
        }
    }
}

pub fn listen_tcp<T: Fn(TcpStream)>(
    addr: &str,
    cb: T
) -> i32 {
    extern "C" fn raw_cb(stream_tid: i32, user_data: i32) -> i32 {
        let cb: &Box<Fn(TcpStream)> = unsafe { &*(
            user_data as *const Box<Fn(TcpStream)>
        ) };
        cb(TcpStream {
            inner: Rc::new(TcpStreamImpl {
                handle: stream_tid
            })
        });
        0
    }

    let f: Box<Box<Fn(TcpStream)>> = Box::new(Box::new(cb));

    unsafe {
        let addr = addr.as_bytes();
        __ice_tcp_listen(
            &addr[0],
            addr.len(),
            raw_cb,
            Box::into_raw(f) as _
        )
    }
}
