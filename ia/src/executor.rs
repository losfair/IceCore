use futures::executor::{Executor, SpawnError};
use futures::task::{LocalMap, Context, Waker, Wake};
use futures::prelude::*;
use std::sync::Arc;
use std::cell::UnsafeCell;

pub struct TaskInfo {
    local_map: *mut LocalMap,
    host: Host,
    fut: UnsafeCell<Box<Future<Item = (), Error = Never> + 'static + Send>>
}

unsafe impl Send for TaskInfo {}
unsafe impl Sync for TaskInfo {}

impl TaskInfo {
    fn new(host: Host, fut: Box<Future<Item = (), Error = Never> + 'static + Send>) -> TaskInfo {
        TaskInfo {
            local_map: Box::into_raw(Box::new(LocalMap::new())),
            host: host,
            fut: UnsafeCell::new(fut)
        }
    }

    fn get_local_map(&self) -> &mut LocalMap {
        unsafe {
            &mut *self.local_map
        }
    }

    fn get_future(&self) -> &mut Box<Future<Item = (), Error = Never> + 'static + Send> {
        unsafe {
            &mut *self.fut.get()
        }
    }
}

impl Drop for TaskInfo {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.local_map);
        }
    }
}

#[derive(Clone)]
pub struct Host {
    inner: Arc<HostImpl>
}

struct HostImpl {

}

impl Host {
    pub fn new() -> Host {
        Host {
            inner: Arc::new(HostImpl {})
        }
    }
}

impl Executor for Host {
    fn spawn(
        &mut self,
        f: Box<Future<Item = (), Error = Never> + 'static + Send>
    ) -> Result<(), SpawnError> {
        let task = Arc::new(TaskInfo::new(self.clone(), f));

        ::schedule(move || {
            TaskInfo::run_once(&task);
        });

        Ok(())
    }
}

impl TaskInfo {
    fn run_once(arc_self: &Arc<Self>) {
        let f = arc_self.get_future();

        let map = arc_self.get_local_map();
        let waker: Waker = arc_self.clone().into();
        let mut host = arc_self.host.clone();

        let mut ctx = Context::new(
            map,
            &waker,
            &mut host
        );
        match f.poll(&mut ctx) {
            Ok(Async::Ready(())) => {},
            Ok(Async::Pending) => {},
            Err(_) => {}
        }
    }
}

impl Wake for TaskInfo {
    fn wake(arc_self: &Arc<Self>) {
        TaskInfo::run_once(&arc_self);
    }
}
