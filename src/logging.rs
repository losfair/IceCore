
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

    pub fn log(&self, msg: Message) {
        let local_time: chrono::DateTime<chrono::Local> = chrono::Local::now();
        let date = local_time.format("%a %b %e %T %Y").to_string();

        let (kind, text) = match msg {
            Message::Info(t) => (Green.paint("[INFO]").to_string(), t),
            Message::Warning(t) => (Yellow.paint("[WARNING]").to_string(), t),
            Message::Error(t) => (Red.paint("[ERROR]").to_string(), t)
        };

        println!("{} {} {}: {}", Cyan.bold().paint(date.as_str()), Style::new().bold().paint(kind), self.module_name, text);
    }
}

pub enum Message {
    Info(String),
    Warning(String),
    Error(String)
}

macro_rules! logger {
    ($name:expr) => (::logging::Logger::new($name))
}

macro_rules! dinfo {
    ($logger:expr, $fmt:expr) => (
        $logger.log(
            ::logging::Message::Info(format!($fmt))
        )
    );
    ($logger:expr, $fmt:expr, $($arg:tt)*) => (
        $logger.log(
            ::logging::Message::Info(format!($fmt, $($arg)*))
        )
    );
}

macro_rules! dwarning {
    ($logger:expr, $fmt:expr) => (
        $logger.log(
            ::logging::Message::Warning(format!($fmt))
        )
    );
    ($logger:expr, $fmt:expr, $($arg:tt)*) => (
        $logger.log(
            ::logging::Message::Warning(format!($fmt, $($arg)*))
        )
    );
}

macro_rules! derror {
    ($logger:expr, $fmt:expr) => (
        $logger.log(
            ::logging::Message::Error(format!($fmt))
        )
    );
    ($logger:expr, $fmt:expr, $($arg:tt)*) => (
        $logger.log(
            ::logging::Message::Error(format!($fmt, $($arg)*))
        )
    );
}

