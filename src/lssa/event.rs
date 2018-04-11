use std::ops::Deref;
use super::app::Application;

pub struct EventInfo {
    pub(super) app_id: usize,
    ev: Box<Event>
}

impl Deref for EventInfo {
    type Target = Event;

    fn deref(&self) -> &Self::Target {
        &*self.ev
    }
}

pub trait Event: Send + 'static {
    fn notify(&self, app: &Application);
}

impl EventInfo {
    pub fn new<T: Event>(app_id: usize, v: T) -> EventInfo {
        EventInfo {
            app_id: app_id,
            ev: Box::new(v)
        }
    }
}
