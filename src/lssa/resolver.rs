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
        app.invoke2(self.cb, self.data, tid as i32);
    }
}

pub struct IoCompleteEvent {
    cb: i32,
    len: i32,
    data: i32
}

impl Event for IoCompleteEvent {
    fn notify(&self, app: &Application) {
        app.invoke2(self.cb, self.data, self.len);
    }
}

impl NativeResolver for LssaResolver {
    fn resolve(&self, module: &str, field: &str) -> Option<NativeEntry> {
        dinfo!(logger!("resolve"), "Resolve: {} {}", module, field);
        if module != "env" {
            return None;
        }

        match field {
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
        self.add_namespace(ns::tcp::TcpNs::new(
            ns::tcp::TcpImpl::new(),
            app.clone()
        ));
    }
}
