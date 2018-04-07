use wasm_core::trans;
use super::app::{Application, AppConfig};
use container::Container;
use super::task::TaskInfo;
use super::event::EventInfo;
use super::control::Control;
use super::stats::{Stats, AppStats};
use futures::Sink;

use std::collections::{BTreeMap, HashMap};

pub struct AppManager {
    container: Container,
    apps: HashMap<String, Application>
}

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
            trans::translate_module_raw(
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

    pub fn dispatch_control(&self, c: Control) {
        match c {
            Control::Event(ev) => {
                let app = self.apps.get(&ev.app_name).unwrap();
                ev.notify(app);
            },
            Control::Stats(mut req) => {
                let mut stats: BTreeMap<String, AppStats> = BTreeMap::new();
                for (k, app) in &self.apps {
                    stats.insert(k.clone(), app.stats());
                }
                req.feedback.start_send(Stats {
                    applications: stats
                }).unwrap();
            }
        }
    }
}
