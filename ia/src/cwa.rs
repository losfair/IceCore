#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::cwa::write_info(&format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => ($crate::cwa::write_info(&format!($fmt, $($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    ($fmt:expr) => ($crate::cwa::write_warning(&format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => ($crate::cwa::write_warning(&format!($fmt, $($arg)*)));
}

#[wasm_import_module = "cwa"]
extern "C" {
    pub fn log_write(level: i32, text_base: *const u8, text_len: usize);
}

pub fn write_info(s: &str) {
    let s = s.as_bytes();
    unsafe {
        log_write(
            6,
            &s[0],
            s.len()
        );
    }
}

pub fn write_warning(s: &str) {
    let s = s.as_bytes();
    unsafe {
        log_write(
            3,
            &s[0],
            s.len()
        );
    }
}
