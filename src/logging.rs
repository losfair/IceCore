use ansi_term::Style;
use ansi_term::Colour::*;
use chrono;

pub struct Logger {
    module_name: &'static str
}

impl Logger {
    pub fn new(module_name: &'static str) -> Logger {
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
