use wasm_core::trans;
use super::app::{Application, AppConfig, AppMigration};
use container::Container;
use super::control::Control;
use super::stats::{Stats, AppStats};
use futures::Sink;
use sha2::Sha256;

use std::collections::BTreeMap;

pub struct AppManager {
    container: Container,
    apps: Vec<AppOrUninitialized>
}

enum AppOrUninitialized {
    App(Application),
    Uninitialized { code: Vec<u8>, config: AppConfig }
}

impl AppManager {
    pub fn new(container: Container) -> AppManager {
        AppManager {
            container: container,
            apps: vec! []
        }
    }

    fn add(&mut self, app_id: usize, app: AppOrUninitialized) {
        assert_eq!(self.apps.len(), app_id);
        self.apps.push(app);
    }

    pub fn basic_activate(container: Container, code: &[u8], config: &AppConfig) -> Application {
        use sha2::Digest;

        let mut hasher = Sha256::default();
        hasher.input(code);
        let mut code_sha256: [u8; 32] = [0; 32];
        code_sha256.copy_from_slice(hasher.result().as_slice());

        Application::new(
            trans::translate_module_raw(
                code,
                Default::default()
            ),
            code,
            code_sha256,
            config.clone(),
            container
        )
    }

    pub fn migrate_away(&mut self, app_id: usize) -> AppMigration {
        let app: &Application = if let AppOrUninitialized::App(ref app) = self.apps[app_id] {
            app
        } else {
            panic!("Expecting an initialized app");
        };

        let mig = app.start_migration();
        let code = app.code.clone();
        let config = app.config.clone();

        self.apps[app_id] = AppOrUninitialized::Uninitialized {
            code: code,
            config: config
        };

        mig
    }

    pub fn activate_migration(&self, app_id: usize, migration: &AppMigration) {
        use std::time::Instant;

        let logger = logger!("AppManager::activate_migration");

        let begin_time = Instant::now();

        let app = if let AppOrUninitialized::Uninitialized { ref code, ref config } = self.apps[app_id] {
            Self::basic_activate(self.container.clone(), code, config)
        } else {
            panic!("Attempting to migrate on an already initialized application");
        };

        dinfo!(logger, "Application {} loaded", app.name);

        app.complete_migration(migration);
        dinfo!(
            logger,
            "Application {} migrated. Total time: {}ms",
            app.name,
            {
                let elapsed = Instant::now().duration_since(begin_time);
                let repr = elapsed.as_secs() * 1000 + (elapsed.subsec_nanos() / 1000000) as u64;
                repr
            }
        );
    }

    pub fn load(&mut self, code: &[u8], app_id: usize, config: AppConfig) {
        use std::time::Instant;

        let logger = logger!("AppManager::load");

        if config.deferred {
            self.add(app_id, AppOrUninitialized::Uninitialized {
                code: code.to_vec(),
                config: config
            });
            return;
        }

        let begin_time = Instant::now();
        let app = Self::basic_activate(self.container.clone(), code, &config);
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

        self.add(app_id, AppOrUninitialized::App(app));
    }

    pub fn dispatch_control(&mut self, c: Control) {
        match c {
            Control::Event(ev) => {
                let app = match self.apps[ev.app_id] {
                    AppOrUninitialized::App(ref v) => v,
                    _ => panic!("Not initialized")
                };
                ev.notify(app);
            },
            Control::Stats(mut req) => {
                let mut stats: BTreeMap<String, AppStats> = BTreeMap::new();
                for app in &self.apps {
                    if let AppOrUninitialized::App(ref app) = *app {
                        let name = app.name.clone();
                        stats.insert(name, app.stats());
                    }
                }
                req.feedback.start_send(Stats {
                    applications: stats
                }).unwrap();
            },
            Control::ActivateMigration { app_id, migration } => {
                self.activate_migration(app_id, &migration);
            },
            Control::MigrateAway { app_id, mut sender } => {
                let mig = self.migrate_away(app_id);
                sender.start_send(mig).unwrap();
            }
        }
    }
}
