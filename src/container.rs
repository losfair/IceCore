use std::sync::{Arc, Mutex};
use std::ops::Deref;

use config::Config;
use lssa::event::EventInfo;

use futures::sync::mpsc::Sender;
use futures::Sink;

#[derive(Clone)]
pub struct Container {
    inner: Arc<ContainerImpl>
}

pub struct ContainerImpl {
    pub config: Arc<Config>,
    event_dispatcher: Mutex<Option<EventDispatcher>>
}

pub struct EventDispatcher {
    sender: Sender<EventInfo>
}

impl Container {
    pub fn new(config: Arc<Config>) -> Container {
        Container {
            inner: Arc::new(ContainerImpl {
                config: config,
                event_dispatcher: Mutex::new(None)
            })
        }
    }

    pub fn dispatch_event(&self, ev: EventInfo) -> Result<(), ()> {
        let mut dispatcher = self.event_dispatcher.lock().unwrap();
        let dispatcher = match *dispatcher {
            Some(ref mut v) => v,
            None => return Err(())
        };

        match dispatcher.sender.start_send(ev) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    pub fn set_event_dispatcher(&self, d: EventDispatcher) {
        let mut dispatcher = self.event_dispatcher.lock().unwrap();
        if dispatcher.is_some() {
            panic!("Attempting to re-set event dispatcher");
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

impl EventDispatcher {
    pub fn new(sender: Sender<EventInfo>) -> EventDispatcher {
        EventDispatcher {
            sender: sender
        }
    }
}
