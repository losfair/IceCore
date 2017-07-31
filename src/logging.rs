use ansi_term::Style;
use ansi_term::Colour::*;
use chrono;

#[derive(Clone)]
pub struct Logger {
    module_name: String
}

impl Logger {
    pub fn new<T>(module_name: T) -> Logger where T: AsRef<str> {
        Logger {
            module_name: module_name.as_ref().to_string()
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
