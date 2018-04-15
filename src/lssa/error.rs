use wasm_core::value::Value;

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Copy, Clone)]
pub enum ErrorCode {
    Success = 0,

    Generic = 1,
    Eof = 2,
    Shutdown = 3,
    PermissionDenied = 4,
    OngoingIo = 5,

    InvalidInput = 6,
    BindFail = 7,

    NotFound = 8
}

impl ErrorCode {
    pub fn to_ret(&self) -> Value {
        Value::I32(self.to_i32())
    }

    pub fn to_i32(&self) -> i32 {
        -(*self as i32)
    }
}

impl From<::std::io::ErrorKind> for ErrorCode {
    fn from(other: ::std::io::ErrorKind) -> ErrorCode {
        use std::io::ErrorKind::*;

        match other {
            NotFound => ErrorCode::NotFound,
            PermissionDenied => ErrorCode::PermissionDenied,
            InvalidInput => ErrorCode::InvalidInput,
            _ => ErrorCode::Generic
        }
    }
}
