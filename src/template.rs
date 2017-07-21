use std;
use tera;
use std::sync::RwLock;
use serde_json;
use logging;

pub struct TemplateStorage {
    engine: RwLock<tera::Tera>
}

impl TemplateStorage {
    pub fn new() -> TemplateStorage {
        TemplateStorage {
            engine: RwLock::new(tera::Tera::default())
        }
    }

    pub fn add(&self, name: &str, content: &str) -> bool {
        let logger = logging::Logger::new("TemplateStorage::add");

        match self.engine.write().unwrap().add_raw_template(name, content) {
            Ok(_) => {
                logger.log(logging::Message::Info(format!("Template {} added", name)));
                true
            },
            Err(e) => {
                logger.log(logging::Message::Error(format!("Unable to add template {}: {:?}", name, e)));
                false
            }
        }
    }

    pub fn render_json(&self, name: &str, data: &str) -> Option<String> {
        let logger = logging::Logger::new("TemplateStorage::render_json");

        let data: serde_json::Value = match serde_json::from_str(data) {
            Ok(d) => d,
            Err(_) => return None
        };
        match self.engine.read().unwrap().render(name, &data) {
            Ok(d) => Some(d),
            Err(e) => {
                logger.log(logging::Message::Error(format!("Unable to render template {}: {:?}", name, e)));
                None
            }
        }
    }
}

