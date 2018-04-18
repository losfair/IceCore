pub mod log;
pub mod runtime;

const MAJOR_VERSION: i32 = 0;
const MINOR_VERSION: i32 = 0;

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
#[allow(dead_code)]
enum ErrorCode {
    UnknownError = -1,
    InvalidArgumentError = -2,
    PermissionDeniedError = -3,
    NotFoundError = -4
}

impl Into<i32> for ErrorCode {
    fn into(self) -> i32 {
        self as i32
    }
}
