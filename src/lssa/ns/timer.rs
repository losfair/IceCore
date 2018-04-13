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

impl TimerImpl {
    pub fn now_millis(&self, ctx: InvokeContext) -> Option<Value> {
        use chrono;
        let utc_time: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        Some(Value::I64(utc_time.timestamp_millis()))
    }

    pub fn set_immediate(&self, ctx: InvokeContext) -> Option<Value> {
        let app_weak = ctx.app.clone();
        let cb_target = ctx.args[0].get_i32().unwrap();
        let cb_data = ctx.args[1].get_i32().unwrap();

        tokio::executor::current_thread::spawn(futures::future::lazy(move || {
            app_weak.upgrade().unwrap().invoke1(
                cb_target,
                cb_data
            );
            Ok(())
        }));

        None
    }
}
