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
    streams: Arc<Mutex<Slab<Option<tokio::net::TcpStream>>>>,
    buffers: Arc<Mutex<Slab<Vec<u8>>>>
}

impl TcpImpl {
    pub fn new() -> TcpImpl {
        TcpImpl {
            streams: Arc::new(Mutex::new(Slab::new())),
            buffers: Arc::new(Mutex::new(Slab::new()))
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

        let app_weak = ctx.app.clone();

        let saddr: SocketAddr = addr.parse().unwrap();
        let listener = tokio::net::TcpListener::bind(&saddr).unwrap();

        let streams = self.streams.clone();

        tokio::executor::current_thread::spawn(
            listener.incoming().for_each(move |s| {
                let stream_id = streams.lock().unwrap().insert(Some(s));

                app_weak.upgrade().unwrap().invoke2(
                    cb_target,
                    cb_data,
                    stream_id as _
                );
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

        let app_weak1 = ctx.app.clone();
        let app_weak2 = ctx.app.clone();

        let data_len = data.len();

        tokio::executor::current_thread::spawn(
            tokio::io::write_all(conn, data.to_vec()).map(move |(a, _)| {
                streams.lock().unwrap()[stream_id] = Some(a);

                app_weak1.upgrade().unwrap().invoke2(
                    cb_target,
                    cb_data,
                    data_len as _
                );
            }).or_else(move |e| {
                derror!(logger!("(app)"), "Write error: {:?}", e);
                app_weak2.upgrade().unwrap().invoke2(
                    cb_target,
                    cb_data,
                    -1
                );
                Ok(())
            })
        );

        Some(Value::I32(0))
    }
}
