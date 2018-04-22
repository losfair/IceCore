use wasm_core::trans;
use super::app::{Application, AppConfig};
use container::Container;
use super::control::Control;
use super::stats::{Stats, AppStats};
use futures::Sink;
use sha2::Sha256;

use std::collections::BTreeMap;

pub struct AppManager {
    container: Container,
    apps: Vec<Application>
}

impl AppManager {
    pub fn new(container: Container) -> AppManager {
        AppManager {
            container: container,
            apps: vec! []
        }
    }

    fn add(&mut self, app_id: usize, app: Application) {
        assert_eq!(self.apps.len(), app_id);
        self.apps.push(app);
    }

    pub fn load(&mut self, code: &[u8], app_id: usize, config: AppConfig) {
        use std::time::Instant;
        use sha2::Digest;

        let logger = logger!("AppManager::load");

        let begin_time = Instant::now();

        let mut hasher = Sha256::default();
        hasher.input(code);
        let mut code_sha256: [u8; 32] = [0; 32];
        code_sha256.copy_from_slice(hasher.result().as_slice());

        let app = Application::new(
            trans::translate_module_raw(
                code,
                Default::default()
            ),
            code_sha256,
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

        self.add(app_id, app);
    }

    pub fn dispatch_control(&self, c: Control) {
        match c {
            Control::Event(ev) => {
                let app = &self.apps[ev.app_id];
                ev.notify(app);
            },
            Control::Stats(mut req) => {
                let mut stats: BTreeMap<String, AppStats> = BTreeMap::new();
                for app in &self.apps {
                    let name = app.name.clone();
                    stats.insert(name, app.stats());
                }
                req.feedback.start_send(Stats {
                    applications: stats
                }).unwrap();
            }
        }
    }
}
