use futures::prelude::*;
use std::sync::Arc;
use std::cell::{RefCell, UnsafeCell};

pub struct TaskInfo {
    fut: UnsafeCell<Box<Future<Item = (), Error = !> + 'static>>
}

unsafe impl Send for TaskInfo {}
unsafe impl Sync for TaskInfo {}

impl TaskInfo {
    fn new(fut: Box<Future<Item = (), Error = !> + 'static>) -> TaskInfo {
        TaskInfo {
            fut: UnsafeCell::new(fut)
        }
    }

    fn get_future(&self) -> &mut Box<Future<Item = (), Error = !> + 'static> {
        unsafe {
            &mut *self.fut.get()
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Host;

impl Host {
    pub fn spawn(f: Box<Future<Item = (), Error = !> + 'static>) {
        let task = Arc::new(TaskInfo::new(f));
        TaskInfo::run_once_next_tick(&task);
    }
}

thread_local! {
    static CURRENT_TASKS: RefCell<Vec<Arc<TaskInfo>>> = RefCell::new(Vec::new());
}

struct CurrentTaskGuard {
    _placeholder: ()
}

impl CurrentTaskGuard {
    fn new(t: Arc<TaskInfo>) -> CurrentTaskGuard {
        CURRENT_TASKS.with(move |tasks| {
            tasks.borrow_mut().push(t);
        });
        CurrentTaskGuard {
            _placeholder: ()
        }
    }
}

impl Drop for CurrentTaskGuard {
    fn drop(&mut self) {
        CURRENT_TASKS.with(move |tasks| {
            tasks.borrow_mut().pop().unwrap();
        });
    }
}

impl TaskInfo {
    pub fn run_once_next_tick(arc_self: &Arc<Self>) {
        run_once_next_tick(arc_self)
    }

    pub fn run_once(arc_self: &Arc<Self>) {
        let f = arc_self.get_future();
        let guard = CurrentTaskGuard::new(arc_self.clone());

        match f.poll() {
            Ok(Async::Ready(())) => {},
            Ok(Async::NotReady) => {},
            Err(_) => {}
        }
    }
}

pub fn current_task() -> Arc<TaskInfo> {
    CURRENT_TASKS.with(move |tasks| {
        tasks.borrow().last().unwrap().clone()
    })
}

pub fn run_once_next_tick(target: &Arc<TaskInfo>) {
    let t = target.clone();
    ::schedule(move || {
        TaskInfo::run_once(&t);
    });
}