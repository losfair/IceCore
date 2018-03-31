use std::any::Any;
use std::ops::Deref;

pub struct TaskInfo {
    pub(crate) app_name: String,
    task: Box<Any + Send>
}

impl Deref for TaskInfo {
    type Target = Any;

    fn deref(&self) -> &Self::Target {
        &*self.task
    }
}

impl TaskInfo {
    pub fn new<S: Into<String>, T: Send + 'static>(app_name: S, v: T) -> TaskInfo {
        TaskInfo {
            app_name: app_name.into(),
            task: Box::new(v)
        }
    }
}

pub struct CallbackTask {
    pub(super) target: i32,
    pub(super) data: i32
}
