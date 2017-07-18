use std;
use tera;
use std::sync::RwLock;
use time;
use std::collections::HashMap;
use serde_json;

pub struct TemplateStorage {
    engine: RwLock<tera::Tera>,
    perf_info: RwLock<HashMap<String, PerfInfo>>
}

pub struct PerfInfo {
    min_render_time: u64,
    max_render_time: u64
}

impl TemplateStorage {
    pub fn new() -> TemplateStorage {
        TemplateStorage {
            engine: RwLock::new(tera::Tera::default()),
            perf_info: RwLock::new(HashMap::new())
        }
    }

    pub fn add(&self, name: &str, content: &str) -> bool {
        match self.engine.write().unwrap().add_raw_template(name, content) {
            Ok(_) => true,
            Err(_) => false
        }
    }

    pub fn render_json(&self, name: &str, data: &str) -> Option<String> {
        let start_time = time::micros();

        let data: serde_json::Value = match serde_json::from_str(data) {
            Ok(d) => d,
            Err(_) => return None
        };
        let output = match self.engine.read().unwrap().render(name, &data) {
            Ok(d) => d,
            Err(e) => {
                println!("Unable to render template {}: {:?}", name, e);
                return None;
            }
        };

        let end_time = time::micros();
        let duration = end_time - start_time;

        let mut update_perf_info = false;

        let mut new_min: u64 = std::u64::MAX;
        let mut new_max: u64 = 0;

        {
            let info = self.perf_info.read().unwrap();
            match info.get(&name.to_string()) {
                Some(v) => {
                    new_min = v.min_render_time;
                    new_max = v.max_render_time;
                },
                None => {}
            };
            
            if duration < new_min {
                update_perf_info = true;
                new_min = duration;
            }
            if duration > new_max {
                update_perf_info = true;
                new_max = duration;
            }
        }

        if update_perf_info {
            self.perf_info.write().unwrap().insert(name.to_string(), PerfInfo {
                min_render_time: new_min,
                max_render_time: new_max
            });
        }

        Some(output)
    }
}

impl PerfInfo {
    fn new() -> PerfInfo {
        PerfInfo {
            min_render_time: std::u64::MAX,
            max_render_time: 0
        }
    }
}
