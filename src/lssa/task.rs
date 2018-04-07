use std::any::Any;
use std::ops::Deref;
use super::app::Application;

pub struct TaskInfo {
    task: Box<Any>
}

impl Deref for TaskInfo {
    type Target = Any;

    fn deref(&self) -> &Self::Target {
        &*self.task
    }
}

impl TaskInfo {
    pub fn new<T: Any>(v: T) -> TaskInfo {
        TaskInfo {
            task: Box::new(v)
        }
    }
}
