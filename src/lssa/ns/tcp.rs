use config::AppPermission;
use super::super::namespace::{InvokeContext, MigrationProvider, Migration};
use wasm_core::value::Value;
use std::net::SocketAddr;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::BTreeMap;
use slab::Slab;

use futures;
use futures::{Future, Stream};
use tokio;
use tokio::prelude::AsyncRead;
use tokio_io::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use super::super::error::ErrorCode;
use super::super::app::ApplicationImpl;

decl_namespace!(
    TcpNs,
    "tcp",
    TcpImpl,
    release_buffer,
    take_buffer,
    connect,
    listen,
    read,
    write,
    destroy
);

#[derive(Serialize, Deserialize, Clone)]
struct TcpMigrationInfo {
    rw_callbacks: Vec<RwCallback>,
    listening_addresses: Vec<(String, RwCallback)>
}

pub struct TcpMigrationProvider;
impl MigrationProvider<TcpNs> for TcpMigrationProvider {
    fn start_migration(target: &TcpNs) -> Option<Migration> {
        Some(Migration::new(&TcpMigrationInfo {
            rw_callbacks: target.provider.rw_callbacks.borrow().iter()
                .map(|(_, b)| *b)
                .collect(),
            listening_addresses: target.provider.listening.borrow().iter()
                .map(|(a, b)| (a.clone(), *b))
                .collect()
        }))
    }

    fn complete_migration(target: &TcpNs, mig: &Migration) {
        let info: TcpMigrationInfo = mig.extract().unwrap();
        for (addr, cb) in &info.listening_addresses {
            target.provider.listen_with_cb(
                target.provider.app.clone(),
                addr,
                cb.cb_target,
                cb.cb_data
            );
        }
        let app = target.provider.app.upgrade().unwrap();
        for rwcb in &info.rw_callbacks {
            app.invoke2(
                rwcb.cb_target,
                rwcb.cb_data,
                -1
            );
        }
    }
}

pub struct TcpImpl {
    app: Weak<ApplicationImpl>,
    streams: Rc<RefCell<Slab<(
        Option<ReadHalf<TcpStream>>,
        Option<WriteHalf<TcpStream>>
    )>>>,
    buffers: Rc<RefCell<Slab<Box<[u8]>>>>,
    rw_callbacks: Rc<RefCell<Slab<RwCallback>>>,
    listening: Rc<RefCell<BTreeMap<String, RwCallback>>>
}

#[derive(Serialize, Deserialize, Copy, Clone)]
struct RwCallback {
    cb_target: i32,
    cb_data: i32
}

impl TcpImpl {
    pub fn new(app: Weak<ApplicationImpl>) -> TcpImpl {
        TcpImpl {
            app: app,
            streams: Rc::new(RefCell::new(Slab::new())),
            buffers: Rc::new(RefCell::new(Slab::new())),
            rw_callbacks: Rc::new(RefCell::new(Slab::new())),
            listening: Rc::new(RefCell::new(BTreeMap::new()))
        }
    }

    fn do_connect(
        &self,
        weak_app: Weak<ApplicationImpl>,
        addr: &str
    ) -> impl Future<Item = tokio::net::TcpStream, Error = ErrorCode> {
        let addr = Rc::new(addr.to_string());
        let streams = self.streams.clone();
        let app_weak1 = weak_app.clone();
        let app_weak2 = weak_app.clone();

        let addr1 = addr.clone();

        futures::future::lazy(move || {
            let app = app_weak1.upgrade().unwrap();
            app.check_permission(&AppPermission::TcpConnectAny)
                .or_else(|_| app.check_permission(&AppPermission::TcpConnect((*addr1).clone())))
                .map_err(|_| {
                    derror!(
                        logger!(&app.name),
                        "TcpConnectAny or TcpConnect({}) permission is required",
                        addr
                    );
                    ErrorCode::PermissionDenied
                })
                .and_then(|_| -> Result<SocketAddr, ErrorCode> {
                    addr1.parse().map_err(|_| ErrorCode::InvalidInput)
                })
        }).and_then(move |addr| {
            tokio::net::TcpStream::connect(&addr)
                .map_err(|e| {
                    derror!(logger!("(app)"), "Connect error: {:?}", e);
                    ErrorCode::Generic
                })
        })
    }

    pub fn connect(&self, ctx: InvokeContext) -> Option<Value> {
        let addr = ctx.extract_str(0, 1);
        let cb_target = ctx.args[2].get_i32().unwrap();
        let cb_data = ctx.args[3].get_i32().unwrap();

        let streams = self.streams.clone();
        let app_weak1 = ctx.app.clone();
        let app_weak2 = ctx.app.clone();

        tokio::executor::current_thread::spawn(
            self.do_connect(ctx.app.clone(), addr)
                .and_then(move |stream| {
                    let (rh, wh) = stream.split();
                    let stream_id = streams.borrow_mut().insert((
                        Some(rh),
                        Some(wh)
                    ));
                    app_weak1.upgrade().unwrap().invoke2(
                        cb_target,
                        cb_data,
                        stream_id as _
                    );
                    Ok(())
                })
                .or_else(move |code| {
                    app_weak2.upgrade().unwrap().invoke2(
                        cb_target,
                        cb_data,
                        code.to_i32()
                    );
                    Ok(())
                })
        );

        None
    }

    fn do_listen(
        &self,
        weak_app: Weak<ApplicationImpl>,
        addr: &str
    ) -> impl Future<Item = impl Stream<Item = tokio::net::TcpStream, Error = ErrorCode>, Error = ErrorCode> {
        let addr = Rc::new(addr.to_string());
        let app_weak1 = weak_app.clone();
        let app_weak2 = weak_app.clone();

        let addr1 = addr.clone();

        futures::future::lazy(move || {
            let app = app_weak1.upgrade().unwrap();
            app.check_permission(&AppPermission::TcpListenAny)
                .or_else(|_| app.check_permission(&AppPermission::TcpListen((*addr1).clone())))
                .map_err(|_| {
                    derror!(
                        logger!(&app.name),
                        "TcpListenAny or TcpListen({}) permission is required",
                        addr
                    );
                    ErrorCode::PermissionDenied
                })
                .and_then(|_| -> Result<SocketAddr, ErrorCode> {
                    addr1.parse().map_err(|_| ErrorCode::InvalidInput)
                })
        }).and_then(move |addr| {
            tokio::net::TcpListener::bind(&addr)
                .map_err(move |e| {
                    let app = app_weak2.upgrade().unwrap();
                    derror!(
                        logger!(&app.name),
                        "Bind failed: {:?}",
                        e
                    );
                    ErrorCode::BindFail
                })
                .map(|listener| {
                    listener.incoming().map_err(|e| {
                        derror!(logger!("(app)"), "Accept error: {:?}", e);
                        ErrorCode::Generic
                    })
                })
        })
    }

    fn listen_with_cb(&self, app: Weak<ApplicationImpl>, addr0: &str, cb_target: i32, cb_data: i32) {
        let streams = self.streams.clone();

        self.listening.borrow_mut().insert(addr0.into(), RwCallback {
            cb_target: cb_target,
            cb_data: cb_data
        });
        let addr = addr0.to_string();
        let listening = self.listening.clone();
        let app_weak1 = app.clone();

        tokio::executor::current_thread::spawn(
            self.do_listen(app, addr0)
                .and_then(move |feed| {
                    feed.for_each(move |stream| {
                        let (rh, wh) = stream.split();
                        let stream_id = streams.borrow_mut().insert((
                            Some(rh),
                            Some(wh)
                        ));

                        app_weak1.upgrade().unwrap().invoke2(
                            cb_target,
                            cb_data,
                            stream_id as _
                        );
                        Ok(())
                    }).map(|_| ())
                })
                .or_else(|_: ErrorCode| {
                    Ok(())
                })
                .then(move |v| {
                    listening.borrow_mut().remove(&addr).unwrap();
                    v
                })
        );
    }

    pub fn listen(&self, ctx: InvokeContext) -> Option<Value> {
        let addr0 = ctx.extract_str(0, 1);
        let cb_target = ctx.args[2].get_i32().unwrap();
        let cb_data = ctx.args[3].get_i32().unwrap();

        self.listen_with_cb(ctx.app.clone(), addr0, cb_target, cb_data);
        Some(ErrorCode::Success.to_ret())
    }

    pub fn destroy(&self, ctx: InvokeContext) -> Option<Value> {
        let stream_id = ctx.args[0].get_i32().unwrap() as usize;
        self.streams.borrow_mut().remove(stream_id);
        None
    }

    pub fn release_buffer(&self, ctx: InvokeContext) -> Option<Value> {
        let buffer_id = ctx.args[0].get_i32().unwrap() as usize;
        self.buffers.borrow_mut().remove(buffer_id);
        None
    }

    pub fn take_buffer(&self, ctx: InvokeContext) -> Option<Value> {
        let buffer_id = ctx.args[0].get_i32().unwrap() as usize;
        let target_ptr = ctx.args[1].get_i32().unwrap() as usize;
        let max_len = ctx.args[2].get_i32().unwrap() as usize;

        let buf = self.buffers.borrow_mut().remove(buffer_id);

        if buf.len() > max_len {
            panic!("take_buffer: buf.len() > max_len");
        }

        let target_mem = &mut ctx.state.get_memory_mut()[target_ptr .. target_ptr + buf.len()];
        target_mem.copy_from_slice(&buf);

        Some(Value::I32(buf.len() as i32))
    }

    pub fn read(&self, ctx: InvokeContext) -> Option<Value> {
        let stream_id = ctx.args[0].get_i32().unwrap() as usize;
        let read_len = ctx.args[1].get_i32().unwrap() as usize;
        let cb_target = ctx.args[2].get_i32().unwrap();
        let cb_data = ctx.args[3].get_i32().unwrap();

        let conn = match self.streams.borrow_mut()[stream_id].0.take() {
            Some(v) => v,
            None => {
                ctx.app.upgrade().unwrap().invoke2(
                    cb_target,
                    cb_data,
                    ErrorCode::OngoingIo.to_i32()
                );
                return None;
            }
        };
        let streams = self.streams.clone();
        let buffers = self.buffers.clone();

        let app_weak1 = ctx.app.clone();
        let app_weak2 = ctx.app.clone();

        tokio::executor::current_thread::spawn(
            AsyncReadFuture::new(conn, read_len)
                .map(move |(stream, data)| {
                    streams.borrow_mut()[stream_id].0 = Some(stream);
                    let buffer_id = buffers.borrow_mut().insert(data);

                    app_weak1.upgrade().unwrap().invoke2(
                        cb_target,
                        cb_data,
                        buffer_id as _
                    );
                })
                .map_err(move |e| {
                    derror!(logger!("(app)"), "Read error: {:?}", e);
                    app_weak2.upgrade().unwrap().invoke2(
                        cb_target,
                        cb_data,
                        -1
                    );
                })
        );

        None
    }

    pub fn write(&self, ctx: InvokeContext) -> Option<Value> {
        let stream_id = ctx.args[0].get_i32().unwrap() as usize;
        let data = ctx.extract_bytes(1, 2);
        let cb_target = ctx.args[3].get_i32().unwrap();
        let cb_data = ctx.args[4].get_i32().unwrap();

        let conn = match self.streams.borrow_mut()[stream_id].1.take() {
            Some(v) => v,
            None => {
                ctx.app.upgrade().unwrap().invoke2(
                    cb_target,
                    cb_data,
                    ErrorCode::OngoingIo.to_i32()
                );
                return None;
            }
        };
        let streams = self.streams.clone();

        let app_weak1 = ctx.app.clone();
        let app_weak2 = ctx.app.clone();

        let data_len = data.len();

        tokio::executor::current_thread::spawn(
            tokio::io::write_all(conn, data.to_vec()).map(move |(a, _)| {
                streams.borrow_mut()[stream_id].1 = Some(a);

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

        None
    }
}

pub struct AsyncReadFuture<T: AsyncRead> {
    inner: Option<T>,
    buf: Vec<u8>
}

impl<T: AsyncRead> AsyncReadFuture<T> {
    fn new(inner: T, len: usize) -> AsyncReadFuture<T> {
        AsyncReadFuture {
            inner: Some(inner),
            buf: vec! [ 0; len ]
        }
    }
}

impl<T: AsyncRead> Future for AsyncReadFuture<T> {
    type Item = (T, Box<[u8]>);
    type Error = tokio::io::Error;

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        let result = self.inner.as_mut().unwrap().poll_read(&mut self.buf);
        match result {
            Ok(tokio::prelude::Async::Ready(n_bytes)) => Ok(
                futures::prelude::Async::Ready(
                    (
                        self.inner.take().unwrap(),
                        self.buf[0..n_bytes].to_vec().into_boxed_slice()
                    )
                )
            ),
            Ok(tokio::prelude::Async::NotReady) => Ok(
                futures::prelude::Async::NotReady
            ),
            Err(e) => Err(e)
        }
    }
}
