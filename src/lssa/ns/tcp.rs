use config::AppPermission;
use super::super::namespace::InvokeContext;
use super::super::event::{EventInfo, Event};
use super::super::control::Control;
use super::super::app::Application;
use wasm_core::value::Value;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::io::Write;
use slab::Slab;

use futures;
use futures::{Future, Stream};
use tokio;

decl_namespace!(
    TcpNs,
    "tcp",
    TcpImpl,
    listen,
    write,
    destroy
);

pub struct TcpImpl {
    streams: Arc<Mutex<Slab<Option<tokio::net::TcpStream>>>>
}

pub struct ConnectEvent {
    cb: i32,
    data: i32,
    stream_id: i32
}

impl Event for ConnectEvent {
    fn notify(&self, app: &Application) {
        app.invoke2(self.cb, self.data, self.stream_id);
    }
}

pub struct IoCompleteEvent {
    cb: i32,
    data: i32,
    len: i32
}

impl Event for IoCompleteEvent {
    fn notify(&self, app: &Application) {
        app.invoke2(self.cb, self.data, self.len);
    }
}

impl TcpImpl {
    pub fn new() -> TcpImpl {
        TcpImpl {
            streams: Arc::new(Mutex::new(Slab::new()))
        }
    }

    pub fn listen(&self, ctx: InvokeContext) -> Option<Value> {
        let addr = ctx.extract_str(0, 1);
        let cb_target = ctx.args[2].get_i32().unwrap();
        let cb_data = ctx.args[3].get_i32().unwrap();

        let app = ctx.app.upgrade().unwrap();
        match app.check_permission(
            &AppPermission::TcpListen(addr.to_string())
        ) {
            Ok(_) => {},
            Err(_) => return Some(Value::I32(-1))
        }

        let container = app.container.clone();
        let app_id = app.id();

        let saddr: SocketAddr = addr.parse().unwrap();
        let listener = tokio::net::TcpListener::bind(&saddr).unwrap();

        let streams = self.streams.clone();

        app.container.thread_pool.spawn(
            listener.incoming().for_each(move |s| {
                let stream_id = streams.lock().unwrap().insert(Some(s));

                container.dispatch_control(Control::Event(EventInfo::new(
                    app_id,
                    ConnectEvent {
                        cb: cb_target,
                        data: cb_data,
                        stream_id: stream_id as _,
                    }
                ))).unwrap();
                Ok(())
            }).map(|_| ()).map_err(move |e| {
                derror!(logger!("(app)"), "Accept error: {:?}", e);
            })
        );

        Some(Value::I32(0))
    }

    pub fn destroy(&self, ctx: InvokeContext) -> Option<Value> {
        let stream_id = ctx.args[0].get_i32().unwrap() as usize;
        self.streams.lock().unwrap().remove(stream_id);
        None
    }

    pub fn write(&self, ctx: InvokeContext) -> Option<Value> {
        let stream_id = ctx.args[0].get_i32().unwrap() as usize;
        let data = ctx.extract_bytes(1, 2);
        let cb_target = ctx.args[3].get_i32().unwrap();
        let cb_data = ctx.args[4].get_i32().unwrap();

        let conn = self.streams.lock().unwrap()[stream_id].take().unwrap();
        let streams = self.streams.clone();

        let app = ctx.app.upgrade().unwrap();

        let app_id = app.id();
        let container1 = app.container.clone();
        let container2 = app.container.clone();

        let data_len = data.len();

        app.container.thread_pool.spawn(
            tokio::io::write_all(conn, data.to_vec()).map(move |(a, _)| {
                streams.lock().unwrap()[stream_id] = Some(a);

                container2.dispatch_control(Control::Event(EventInfo::new(
                    app_id,
                    IoCompleteEvent {
                        cb: cb_target,
                        len: data_len as _,
                        data: cb_data
                    }
                ))).unwrap();
            }).or_else(move |e| {
                derror!(logger!("(app)"), "Write error: {:?}", e);
                container1.dispatch_control(Control::Event(EventInfo::new(
                    app_id,
                    IoCompleteEvent {
                        cb: cb_target,
                        len: -1,
                        data: cb_data
                    }
                ))).unwrap();
                Ok(())
            })
        );

        Some(Value::I32(0))
    }
}
