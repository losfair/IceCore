#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::cwa::log::write(
        $crate::cwa::log::Level::Info,
        &format!($fmt)
    ));
    ($fmt:expr, $($arg:tt)*) => ($crate::cwa::log::write(
        $crate::cwa::log::Level::Info,
        &format!($fmt, $($arg)*)
    ));
}

#[macro_export]
macro_rules! eprintln {
    ($fmt:expr) => ($crate::cwa::log::write(
        $crate::cwa::log::Level::Warning,
        &format!($fmt)
    ));
    ($fmt:expr, $($arg:tt)*) => ($crate::cwa::log::write(
        $crate::cwa::log::Level::Warning,
        &format!($fmt, $($arg)*)
    ));
}
