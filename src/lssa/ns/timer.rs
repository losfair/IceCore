use super::super::namespace::InvokeContext;
use super::super::event::{EventInfo, Event};
use super::super::control::Control;
use super::super::app::Application;
use wasm_core::value::Value;

use futures;
use tokio;

decl_namespace!(
    TimerNs,
    "timer",
    TimerImpl,
    now_millis,
    set_immediate
);

pub struct TimerImpl;

pub struct TimeoutEvent {
    cb: i32,
    data: i32
}

impl Event for TimeoutEvent {
    fn notify(&self, app: &Application) {
        app.invoke1(self.cb, self.data);
    }
}

impl TimerImpl {
    pub fn now_millis(&self, ctx: InvokeContext) -> Option<Value> {
        use chrono;
        let utc_time: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        Some(Value::I64(utc_time.timestamp_millis()))
    }

    pub fn set_immediate(&self, ctx: InvokeContext) -> Option<Value> {
        let ev = TimeoutEvent {
            cb: ctx.args[0].get_i32().unwrap(),
            data: ctx.args[1].get_i32().unwrap()
        };
        let app = ctx.app.upgrade().unwrap();
        let container = app.container.clone();
        let app_id = app.id();

        tokio::executor::current_thread::spawn(futures::future::lazy(move || {
            container.dispatch_control(Control::Event(EventInfo::new(
                app_id,
                ev
            ))).unwrap();
            Ok(())
        }));

        None
    }
}
