use std::ops::Deref;
use super::app::Application;

pub struct EventInfo {
    pub(super) app_name: String,
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
    pub fn new<S: Into<String>, T: Event>(app_name: S, v: T) -> EventInfo {
        EventInfo {
            app_name: app_name.into(),
            ev: Box::new(v)
        }
    }
}
