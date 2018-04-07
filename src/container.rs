use std::sync::{Arc, Mutex};
use std::ops::Deref;

use config::Config;
use lssa::control::Control;

use futures::sync::mpsc::Sender;
use futures::Sink;

use tokio::executor::thread_pool::ThreadPool;

#[derive(Clone)]
pub struct Container {
    inner: Arc<ContainerImpl>
}

pub struct ContainerImpl {
    pub config: Arc<Config>,
    pub thread_pool: Arc<ThreadPool>,
    control_dispatcher: Mutex<Option<ControlDispatcher>>
}

pub struct ControlDispatcher {
    sender: Sender<Control>
}

impl Container {
    pub fn new(config: Arc<Config>) -> Container {
        Container {
            inner: Arc::new(ContainerImpl {
                config: config,
                thread_pool: Arc::new(ThreadPool::new()),
                control_dispatcher: Mutex::new(None)
            })
        }
    }

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
