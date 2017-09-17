use super::{HtmlTemplateEngine, HtmlTemplateError};
use tera;
use serde_json;

pub struct TeraEngine {
    engine: tera::Tera
}

impl TeraEngine {
    pub fn new() -> TeraEngine {
        TeraEngine {
            engine: tera::Tera::default()
        }
    }
}

impl HtmlTemplateEngine for TeraEngine {
    fn add(&mut self, name: &str, content: &str) -> Result<(), HtmlTemplateError> {
        self.engine.add_raw_template(name, content)?;
        Ok(())
    }

    fn render(&self, name: &str, data: &serde_json::Value) -> Result<String, HtmlTemplateError> {
        Ok(self.engine.render(name, data)?)
    }
}
