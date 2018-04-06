use std::any::Any;
use std::ops::Deref;
use super::app::Application;

pub struct TaskInfo {
    pub(crate) app_name: String,
    task: Box<Task>
}

pub trait Task: Send + 'static {
}

impl Deref for TaskInfo {
    type Target = Task;

    fn deref(&self) -> &Self::Target {
        &*self.task
    }
}

impl TaskInfo {
    pub fn new<S: Into<String>, T: Task>(app_name: S, v: T) -> TaskInfo {
        TaskInfo {
            app_name: app_name.into(),
            task: Box::new(v)
        }
    }
}
