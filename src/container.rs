use std::sync::{Arc, Mutex, RwLock};
use std::ops::Deref;
use std::collections::BTreeMap;

use config::Config;
use lssa::control::Control;

use futures::sync::mpsc::Sender;
use futures::Sink;

#[derive(Clone)]
pub struct Container {
    inner: Arc<ContainerImpl>
}

pub struct ContainerImpl {
    pub config_state: RwLock<ConfigState>,
    control_dispatcher: Mutex<Option<ControlDispatcher>>
}

pub struct ControlDispatcher {
    sender: Sender<Control>
}

pub struct ConfigState {
    pub config: Config,
    pub app_name_to_id: BTreeMap<String, usize>
}

impl Container {
    pub fn new(config: Config) -> Container {
        let app_name_to_id = config.applications.iter()
            .enumerate()
            .map(|(i, app)| (app.name.clone(), i))
            .collect();

        Container {
            inner: Arc::new(ContainerImpl {
                config_state: RwLock::new(ConfigState {
                    config: config,
                    app_name_to_id: app_name_to_id
                }),
                control_dispatcher: Mutex::new(None)
            })
        }
    }

    pub fn lookup_app_id_by_name(&self, name: &str) -> Option<usize> {
        let cs = self.config_state.read().unwrap();
        cs.app_name_to_id.get(name).map(|v| *v)
    }

    #[allow(dead_code)]
    pub fn dispatch_control(&self, c: Control) -> Result<(), ()> {
        let mut dispatcher = self.control_dispatcher.lock().unwrap();
        let dispatcher = match *dispatcher {
            Some(ref mut v) => v,
            None => return Err(())
        };

        match dispatcher.sender.start_send(c) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    pub fn set_control_dispatcher(&self, d: ControlDispatcher) {
        let mut dispatcher = self.control_dispatcher.lock().unwrap();
        if dispatcher.is_some() {
            panic!("Attempting to re-set control dispatcher");
        }
        *dispatcher = Some(d);
    }
}

impl Deref for Container {
    type Target = ContainerImpl;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl ControlDispatcher {
    pub fn new(sender: Sender<Control>) -> ControlDispatcher {
        ControlDispatcher {
            sender: sender
        }
    }
}
