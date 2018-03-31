use wasm_translator;
use super::app::{Application, AppConfig};
use container::Container;
use super::task::TaskInfo;

use std::collections::HashMap;

pub struct AppManager {
    container: Container,
    apps: HashMap<String, Application>
}

// FIXME: Is this correct?
unsafe impl Send for AppManager {}

impl AppManager {
    pub fn new(container: Container) -> AppManager {
        AppManager {
            container: container,
            apps: HashMap::new()
        }
    }

    pub fn add(&mut self, name: String, app: Application) {
        self.apps.insert(name, app);
    }

    pub fn load(&mut self, name: String, code: &[u8], config: AppConfig) {
        let app = Application::new(
            wasm_translator::translate_module_raw(
                code,
                Default::default()
            ),
            config,
            self.container.clone()
        );
        app.initialize(None);
        self.add(name, app);
    }

    pub fn invoke_dispatch(&self, task: TaskInfo) {
        let app = self.apps.get(&task.app_name).unwrap();
        let task_id = app.add_task(task);
        app.invoke_inner_dispatcher_on_task(task_id);
    }
}
