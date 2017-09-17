pub mod tera;
pub mod api;

use std;
use std::io::Read;
use std::fs::File;
use serde_json;

pub trait HtmlTemplateEngine {
    fn add(&mut self, name: &str, content: &str) -> Result<(), HtmlTemplateError>;
    fn render(&self, name: &str, data: &serde_json::Value) -> Result<String, HtmlTemplateError>;

    fn add_file(&mut self, path: &str) -> Result<(), HtmlTemplateError> {
        let mut f = File::open(path)?;
        let mut content = String::new();

        f.read_to_string(&mut content)?;
        self.add(&path, content.as_str())
    }
}

pub enum HtmlTemplateError {
    Other(String)
}

impl<T> From<T> for HtmlTemplateError where T: std::error::Error {
    fn from(other: T) -> HtmlTemplateError {
        HtmlTemplateError::Other(other.description().to_string())
    }
}

impl Into<String> for HtmlTemplateError {
    fn into(self) -> String {
        match self {
            HtmlTemplateError::Other(v) => v
        }
    }
}
