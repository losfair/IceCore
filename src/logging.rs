
use ansi_term::Style;
use ansi_term::Colour::*;
use chrono;

#[derive(Clone)]
pub struct Logger<'a> {
    module_name: &'a str
}

impl<'a> Logger<'a> {
    pub fn new(module_name: &'a str) -> Logger<'a> {
        Logger {
            module_name: module_name
        }
    }

    pub fn log<M: AsRef<str>>(&self, level: Level, text: M) {
        let local_time: chrono::DateTime<chrono::Local> = chrono::Local::now();
        let date = local_time.format("%a %b %e %T %Y").to_string();

        let kind = match level {
            Level::Info => Green.paint("[INFO]").to_string(),
            Level::Warning => Yellow.paint("[WARNING]").to_string(),
            Level::Error => Red.paint("[ERROR]").to_string()
        };

        println!(
            "{} {} {}: {}",
            Cyan.bold().paint(date.as_str()),
            Style::new().bold().paint(kind),
            self.module_name,
            text.as_ref()
        );
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Level {
    Info,
    Warning,
    Error
}

macro_rules! logger {
    ($name:expr) => (::logging::Logger::new($name))
}

macro_rules! dinfo {
    ($logger:expr, $fmt:expr) => (
        $logger.log(
            ::logging::Level::Info,
            format!($fmt)
        )
    );
    ($logger:expr, $fmt:expr, $($arg:tt)*) => (
        $logger.log(
            ::logging::Level::Info,
            format!($fmt, $($arg)*)
        )
    );
}

macro_rules! dwarning {
    ($logger:expr, $fmt:expr) => (
        $logger.log(
            ::logging::Level::Warning,
            format!($fmt)
        )
    );
    ($logger:expr, $fmt:expr, $($arg:tt)*) => (
        $logger.log(
            ::logging::Level::Warning,
            format!($fmt, $($arg)*)
        )
    );
}

macro_rules! derror {
    ($logger:expr, $fmt:expr) => (
        $logger.log(
            ::logging::Level::Error,
            format!($fmt)
        )
    );
    ($logger:expr, $fmt:expr, $($arg:tt)*) => (
        $logger.log(

            ::logging::Level::Error,
            format!($fmt, $($arg)*)
        )
    );
}

