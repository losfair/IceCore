use wasm_core::executor::{NativeResolver, NativeEntry};
use wasm_core::value::Value;
use super::app::ApplicationImpl;
use std::rc::Weak;

pub struct LssaResolver {
    app: Weak<ApplicationImpl>
}

impl NativeResolver for LssaResolver {
    fn resolve(&self, module: &str, field: &str) -> Option<NativeEntry> {
        eprintln!("Resolve: {} {}", module, field);
        if module != "env" {
            return None;
        }

        let app = self.app.clone();

        match field {
            "__ice_drop_task" => Some(Box::new(move |_, args| {
                let task_id = args[0].get_i32()?;
                let app = app.upgrade().unwrap();
                app.tasks.borrow_mut().remove(task_id as usize);
                Ok(None)
            })),
            _ => None
        }
    }
}

impl LssaResolver {
    pub fn new(app: Weak<ApplicationImpl>) -> LssaResolver {
        LssaResolver {
            app: app
        }
    }
}
