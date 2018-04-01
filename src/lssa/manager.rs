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

    pub fn add(&mut self, app: Application) {
        let name = app.name.clone();
        self.apps.insert(name, app);
    }

    pub fn load(&mut self, code: &[u8], config: AppConfig) {
        use std::time::Instant;

        let logger = logger!("AppManager::load");

        let begin_time = Instant::now();

        let app = Application::new(
            wasm_translator::translate_module_raw(
                code,
                Default::default()
            ),
            config,
            self.container.clone()
        );
        dinfo!(logger, "Application {} loaded", app.name);

        app.initialize(None);
        dinfo!(
            logger,
            "Application {} initialized. Total time: {}ms",
            app.name,
            {
                let elapsed = Instant::now().duration_since(begin_time);
                let repr = elapsed.as_secs() * 1000 + (elapsed.subsec_nanos() / 1000000) as u64;
                repr
            }
        );

        self.add(app);
    }

    pub fn invoke_dispatch(&self, task: TaskInfo) {
        let app = self.apps.get(&task.app_name).unwrap();
        let task_id = app.add_task(task);
        app.invoke_inner_dispatcher_on_task(task_id);
    }
}
