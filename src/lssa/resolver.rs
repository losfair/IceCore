use wasm_core::executor::{NativeResolver, NativeEntry};
use wasm_core::value::Value;
use super::app::ApplicationImpl;
use super::task::{TaskInfo, CallbackTask};
use std::rc::Weak;
use std::time::{Duration, Instant};
use std::mem::transmute;
use tokio;

use futures;
use futures::Future;

pub struct LssaResolver {
    app: Weak<ApplicationImpl>
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

                tokio::spawn(futures::future::ok(()).map(move |_| {
                    container.dispatch(TaskInfo::new(
                        name,
                        CallbackTask {
                            target: cb_target,
                            data: cb_data
                        }
                    )).unwrap();
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

                tokio::spawn(tokio::timer::Delay::new(
                    Instant::now() + Duration::from_millis(timeout as _)
                ).map(move |_| {
                    container.dispatch(TaskInfo::new(
                        name,
                        CallbackTask {
                            target: cb_target,
                            data: cb_data
                        }
                    )).unwrap();
                    ()
                }).map_err(|e| {
                    derror!(logger!("timer"), "{:?}", e);
                    ()
                }));
                Ok(None)
            })),
            "__ice_try_unwrap_callback_task" => Some(Box::new(move |state, args| {
                let task_id = args[0].get_i32()? as usize;
                let target_ptr = args[1].get_i32()? as usize;
                let data_ptr = args[2].get_i32()? as usize;

                let mem = state.get_memory_mut();

                let app = app.upgrade().unwrap();
                let tasks = app.tasks.borrow();
                let task = &tasks[task_id];

                Ok(Some(match task.downcast_ref::<CallbackTask>() {
                    Some(v) => {
                        let target_v = unsafe {
                            transmute::<i32, [u8; 4]>(v.target)
                        };
                        let data_v = unsafe {
                            transmute::<i32, [u8; 4]>(v.data)
                        };
                        mem[target_ptr .. target_ptr + 4].copy_from_slice(&target_v);
                        mem[data_ptr .. data_ptr + 4].copy_from_slice(&data_v);
                        Value::I32(0)
                    },
                    None => Value::I32(1)
                }))
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
