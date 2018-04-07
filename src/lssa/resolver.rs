use wasm_core::executor::{NativeResolver, NativeEntry};
use wasm_core::value::Value;
use super::app::{Application, ApplicationImpl};
use super::task::{TaskInfo, Task};
use super::event::{EventInfo, Event};
use super::control::Control;
use std::rc::Weak;
use std::time::{Duration, Instant};
use std::mem::transmute;
use tokio;

use futures;
use futures::Future;

pub struct LssaResolver {
    app: Weak<ApplicationImpl>
}

pub struct TimeoutEvent {
    cb: i32,
    data: i32
}

impl Event for TimeoutEvent {
    fn notify(&self, app: &Application) {
        app.invoke1(self.cb, self.data);
    }
}

impl NativeResolver for LssaResolver {
    fn resolve(&self, module: &str, field: &str) -> Option<NativeEntry> {
        dinfo!(logger!("resolve"), "Resolve: {} {}", module, field);
        if module != "env" {
            return None;
        }

        let app = self.app.clone();

        match field {
            "__ice_drop_task" => Some(Box::new(move |_, args| {
                let task_id = args[0].get_i32()?;
                let app = app.upgrade().unwrap();
                app.tasks.borrow_mut().remove(task_id as usize);
                Ok(None)
            })),
            "__ice_log" => Some(Box::new(move |state, args| {
                let mem = state.get_memory();
                let str_base = args[0].get_i32()? as usize;
                let str_len = args[1].get_i32()? as usize;

                let text = ::std::str::from_utf8(
                    &mem[str_base .. str_base + str_len]
                ).unwrap();

                let app = app.upgrade().unwrap();

                dinfo!(logger!(&app.name), "{}", text);
                Ok(None)
            })),
            "__ice_request_instant" => Some(Box::new(move |state, args| {
                let cb_target = args[0].get_i32()?;
                let cb_data = args[1].get_i32()?;

                let app = app.upgrade().unwrap();
                let container = app.container.clone();
                let name = app.name.clone();

                tokio::executor::current_thread::spawn(futures::future::ok(()).map(move |_| {
                    container.dispatch_control(Control::Event(EventInfo::new(
                        name,
                        TimeoutEvent {
                            cb: cb_target,
                            data: cb_data
                        }
                    ))).unwrap();
                    ()
                }));
                Ok(None)
            })),
            "__ice_request_timeout" => Some(Box::new(move |state, args| {
                let timeout = args[0].get_i64()?;
                let cb_target = args[1].get_i32()?;
                let cb_data = args[2].get_i32()?;

                let app = app.upgrade().unwrap();
                let container = app.container.clone();
                let name = app.name.clone();

                let tpool = container.thread_pool.clone();

                tokio::executor::current_thread::spawn(tokio::timer::Delay::new(
                    Instant::now() + Duration::from_millis(timeout as _)
                ).map(move |_| {
                    container.dispatch_control(Control::Event(EventInfo::new(
                        name,
                        TimeoutEvent {
                            cb: cb_target,
                            data: cb_data
                        }
                    ))).unwrap();
                    ()
                }).map_err(|e| {
                    derror!(logger!("timer"), "{:?}", e);
                    ()
                }));
                Ok(None)
            })),
            "__ice_current_time_ms" => Some(Box::new(|_, _| {
                use chrono;
                let utc_time: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
                Ok(Some(Value::I64(utc_time.timestamp_millis())))
            })),
            _ => None
        }
    }
}

impl LssaResolver {
    pub fn new(app: Weak<ApplicationImpl>) -> LssaResolver {
        LssaResolver {
            app: app
        }
    }
}
