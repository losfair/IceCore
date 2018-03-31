use std::sync::Arc;

use container::{Container, TaskDispatcher};
use config::Config;
use lssa;
use lssa::task::TaskInfo;
use lssa::manager::AppManager;

use futures;
use futures::Future;
use futures::Stream;
//use futures::{StreamExt, FutureExt};

pub struct Server {
    container: Container
}

impl Server {
    pub fn new(config: Config) -> Server {
        Server {
            container: Container::new(Arc::new(config))
        }
    }

    pub fn run_apps(&self) -> impl Future<Item = (), Error = ()> + Send {
        let (tx, rx) = futures::sync::mpsc::channel::<TaskInfo>(4096);
        self.container.set_task_dispatcher(TaskDispatcher::new(tx));

        let container = self.container.clone();

        futures::future::ok(()).map(move |_| {
            let mut manager = AppManager::new(container.clone());
            load_apps_from_config(
                &mut manager,
                &*container.config
            );
            manager
        }).then(move |manager: Result<AppManager, ()>| {
            let manager = manager.unwrap();
            rx.for_each(move |task| {
                use std::panic::{catch_unwind, AssertUnwindSafe};
                let maybe_err = catch_unwind(AssertUnwindSafe(|| manager.invoke_dispatch(task)));
                if maybe_err.is_err() {
                    derror!(logger!("invoke_dispatch"), "Unknown error");
                }
                Ok(())
            }).map(|_| ())
        })
    }
}

fn load_apps_from_config(manager: &mut AppManager, config: &Config) {
    use std::fs::File;
    use std::io::Read;

    for app in &config.applications {
        let mut code_file = match File::open(&app.path) {
            Ok(v) => v,
            Err(e) => {
                dwarning!(
                    logger!("load_apps_from_config"),
                    "Unable to load app `{}`: {:?}",
                    app.name,
                    e
                );
                continue;
            }
        };
        let mut code: Vec<u8> = Vec::new();
        code_file.read_to_end(&mut code).unwrap();

        let app_config = lssa::app::AppConfig {
            mem_default: app.memory.min,
            mem_max: app.memory.max,
            name: app.name.clone()
        };
        manager.load(&code, app_config);
    }
}
