use std::sync::{Arc, Mutex};
use std::ops::Deref;

use config::Config;
use lssa::task::TaskInfo;

use futures::sync::mpsc::Sender;
use futures::Sink;

#[derive(Clone)]
pub struct Container {
    inner: Arc<ContainerImpl>
}

pub struct ContainerImpl {
    pub config: Arc<Config>,
    task_dispatcher: Mutex<Option<TaskDispatcher>>
}

pub struct TaskDispatcher {
    sender: Sender<TaskInfo>
}

impl Container {
    pub fn new(config: Arc<Config>) -> Container {
        Container {
            inner: Arc::new(ContainerImpl {
                config: config,
                task_dispatcher: Mutex::new(None)
            })
        }
    }

    pub fn dispatch(&self, task: TaskInfo) -> Result<(), ()> {
        let mut dispatcher = self.task_dispatcher.lock().unwrap();
        let dispatcher = match *dispatcher {
            Some(ref mut v) => v,
            None => return Err(())
        };

        match dispatcher.sender.start_send(task) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    pub fn set_task_dispatcher(&self, d: TaskDispatcher) {
        let mut dispatcher = self.task_dispatcher.lock().unwrap();
        if dispatcher.is_some() {
            panic!("Attempting to re-set task dispatcher");
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

impl TaskDispatcher {
    pub fn new(sender: Sender<TaskInfo>) -> TaskDispatcher {
        TaskDispatcher {
            sender: sender
        }
    }
}
