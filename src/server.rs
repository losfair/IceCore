use std::sync::Arc;

use container::{Container, ControlDispatcher};
use config::Config;
use lssa;
use lssa::event::EventInfo;
use lssa::control::Control;
use lssa::manager::AppManager;

use futures;
use futures::Future;
use futures::Stream;
use futures::Sink;
//use futures::{StreamExt, FutureExt};

pub struct Server {
    container: Container
}

impl Server {
    pub fn new(config: Config) -> Server {
        Server {
            container: Container::new(config)
        }
    }

    fn launch_manager(container: Container) -> futures::sync::mpsc::Sender<Control> {
        let (tx, rx) = futures::sync::mpsc::channel(4096);
        ::std::thread::spawn(move || {
            ::tokio::executor::current_thread::block_on_all(
                futures::future::ok(()).map(move |_| {
                    let mut manager = AppManager::new(container.clone());
                    load_apps_from_config(
                        &mut manager,
                        &container.config_state.read().unwrap().config
                    );
                    manager
                }).map(move |manager| {
                    rx.for_each(move |c| {
                        manager.dispatch_control(c);
                        Ok(())
                    })
                }).flatten().map_err(|_: ()| ())
            ).unwrap();
        });
        tx
    }

    pub fn run_apps(&self) -> impl Future<Item = (), Error = ()> {
        let (tx, rx) = futures::sync::mpsc::channel::<Control>(4096);
        self.container.set_control_dispatcher(ControlDispatcher::new(tx));

        let container = self.container.clone();
        let mut control_sender = Self::launch_manager(container);

        futures::future::ok(()).then(move |_: Result<(), ()>| {
            rx.for_each(move |c| {
                control_sender.start_send(c).unwrap();
                Ok(())
            }).map(|_| ()).map_err(|_| ())
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
