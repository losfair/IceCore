use std::any::Any;
use std::ops::Deref;

pub struct TaskInfo {
    task: Box<Any>
}

impl Deref for TaskInfo {
    type Target = Any;

    fn deref(&self) -> &Self::Target {
        &*self.task
    }
}

#[allow(dead_code)]
impl TaskInfo {
    pub fn new<T: Any>(v: T) -> TaskInfo {
        TaskInfo {
            task: Box::new(v)
        }
    }
}
