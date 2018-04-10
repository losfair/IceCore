use wasm_core::executor::{NativeResolver, NativeEntry};
use wasm_core::value::Value;
use super::app::{Application, ApplicationImpl};
use super::task::TaskInfo;
use super::event::{EventInfo, Event};
use super::control::Control;
use config::AppPermission;
use std::rc::Weak;
use std::time::{Duration, Instant};
use std::mem::transmute;
use std::cell::RefCell;
use std::collections::BTreeMap;
use super::tcp;
use super::namespace::Namespace;
use tokio;

use futures;
use futures::Future;
use futures::Stream;

pub struct LssaResolver {
    app: Weak<ApplicationImpl>,
    namespaces: BTreeMap<String, Box<Namespace>>
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

pub struct ConnectEvent {
    cb: i32,
    stream: RefCell<Option<tcp::TcpConnection>>,
    data: i32
}

impl Event for ConnectEvent {
    fn notify(&self, app: &Application) {
        let tid = app.add_task(TaskInfo::new(self.stream.borrow_mut().take().unwrap()));
        app.invoke2(self.cb, tid as i32, self.data);
    }
}

pub struct IoCompleteEvent {
    cb: i32,
    len: i32,
    data: i32
}

impl Event for IoCompleteEvent {
    fn notify(&self, app: &Application) {
        app.invoke2(self.cb, self.len, self.data);
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
            "__ice_tcp_listen" => Some(Box::new(move |state, args| {
                let mem = state.get_memory();

                let addr_base = args[0].get_i32()? as usize;
                let addr_len = args[1].get_i32()? as usize;

                let cb_target = args[2].get_i32()?;
                let cb_data = args[3].get_i32()?;

                let addr = ::std::str::from_utf8(
                    &mem[addr_base .. addr_base + addr_len]
                ).unwrap();

                let app = app.upgrade().unwrap();
                match app.check_permission(
                    &AppPermission::TcpListen(addr.to_string())
                ) {
                    Ok(_) => {},
                    Err(_) => return Ok(Some(Value::I32(-1)))
                }

                let container = app.container.clone();
                let name1 = app.name.clone();
                let name2 = app.name.clone();

                app.container.thread_pool.spawn(
                    tcp::listen(addr).for_each(move |s| {
                        container.dispatch_control(Control::Event(EventInfo::new(
                            name1.clone(),
                            ConnectEvent {
                                cb: cb_target,
                                stream: RefCell::new(Some(
                                    tcp::TcpConnection::new(s)
                                )),
                                data: cb_data
                            }
                        ))).unwrap();
                        Ok(())
                    }).map(|_| ()).map_err(move |e| {
                        derror!(logger!(&name2), "Accept error: {:?}", e);
                    })
                );

                Ok(Some(Value::I32(0)))
            })),
            "__ice_tcp_write" => Some(Box::new(move |state, args| {
                let mem = state.get_memory();

                let stream_tid = args[0].get_i32()? as usize;
                let data_base = args[1].get_i32()? as usize;
                let data_len = args[2].get_i32()? as usize;
                let cb_target = args[3].get_i32()?;
                let cb_data = args[4].get_i32()?;

                let data = mem[data_base .. data_base + data_len].to_vec();

                let app = app.upgrade().unwrap();

                let tasks = app.tasks.borrow();

                let conn: &tcp::TcpConnection = tasks[stream_tid].downcast_ref().unwrap();

                let app_name1 = app.name.clone();
                let app_name2 = app.name.clone();
                let container1 = app.container.clone();
                let container2 = app.container.clone();

                app.container.thread_pool.spawn(
                    conn.write(data).map_err(move |e| {
                        derror!(logger!(&app_name1), "Write error: {:?}", e);
                        container1.dispatch_control(Control::Event(EventInfo::new(
                            app_name1,
                            IoCompleteEvent {
                                cb: cb_target,
                                len: -1,
                                data: cb_data
                            }
                        ))).unwrap();
                    }).map(move |_| {
                        container2.dispatch_control(Control::Event(EventInfo::new(
                            app_name2,
                            IoCompleteEvent {
                                cb: cb_target,
                                len: data_len as _,
                                data: cb_data
                            }
                        ))).unwrap();
                    })
                );

                Ok(Some(Value::I32(0)))
            })),
            _ => {
                let full_path = match field.split("__ice_").nth(1) {
                    Some(v) => v,
                    None => return None
                };
                let mut parts = full_path.splitn(2, '_');
                let ns_name = match parts.next() {
                    Some(v) => v,
                    None => return None
                };
                let field_name = match parts.next() {
                    Some(v) => v,
                    None => return None
                };
                let ns = match self.namespaces.get(ns_name) {
                    Some(v) => v,
                    None => return None
                };
                ns.dispatch(field_name)
            }
        }
    }
}

impl LssaResolver {
    pub fn new(app: Weak<ApplicationImpl>) -> LssaResolver {
        LssaResolver {
            app: app,
            namespaces: BTreeMap::new()
        }
    }

    pub fn add_namespace<T: Namespace>(&mut self, ns: T) {
        let prefix = ns.prefix().to_string();
        self.namespaces.insert(prefix, Box::new(ns));
    }

    pub fn init_default_namespaces(&mut self) {
        use super::ns;
        let app = self.app.clone();

        self.add_namespace(ns::timer::TimerNs::new(
            ns::timer::TimerImpl,
            app.clone()
        ));
        self.add_namespace(ns::logging::LoggingNs::new(
            ns::logging::LoggingImpl,
            app.clone()
        ));
    }
}
