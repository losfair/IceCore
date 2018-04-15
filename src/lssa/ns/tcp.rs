use config::AppPermission;
use super::super::namespace::InvokeContext;
use wasm_core::value::Value;
use std::net::SocketAddr;
use std::rc::Rc;
use std::cell::RefCell;
use slab::Slab;

use futures;
use futures::{Future, Stream};
use tokio;
use tokio::prelude::AsyncRead;
use tokio_io::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use super::super::error::ErrorCode;

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

pub struct TcpImpl {
    streams: Rc<RefCell<Slab<(
        Option<ReadHalf<TcpStream>>,
        Option<WriteHalf<TcpStream>>
    )>>>,
    buffers: Rc<RefCell<Slab<Box<[u8]>>>>
}

impl TcpImpl {
    pub fn new() -> TcpImpl {
        TcpImpl {
            streams: Rc::new(RefCell::new(Slab::new())),
            buffers: Rc::new(RefCell::new(Slab::new()))
        }
    }

    pub fn connect(&self, ctx: InvokeContext) -> Option<Value> {
        let addr = ctx.extract_str(0, 1);
        let cb_target = ctx.args[2].get_i32().unwrap();
        let cb_data = ctx.args[3].get_i32().unwrap();

        let app = ctx.app.upgrade().unwrap();
        match app.check_permission(&AppPermission::TcpConnectAny)
            .or_else(|_| app.check_permission(&AppPermission::TcpConnect(addr.to_string()))) {
                Ok(_) => {},
                Err(_) => {
                    derror!(
                        logger!(&app.name),
                        "TcpConnectAny or TcpConnect({}) permission is required",
                        addr
                    );
                    return Some(ErrorCode::PermissionDenied.to_ret());
                }
            }

        let saddr: SocketAddr = addr.parse().unwrap();
        let streams = self.streams.clone();
        let app_weak1 = ctx.app.clone();
        let app_weak2 = ctx.app.clone();

        tokio::executor::current_thread::spawn(
            tokio::net::TcpStream::connect(&saddr)
                .map(move |stream| {
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
                })
                .or_else(move |e| {
                    derror!(logger!("(app)"), "Connect error: {:?}", e);
                    app_weak2.upgrade().unwrap().invoke2(
                        cb_target,
                        cb_data,
                        -1
                    );
                    Ok(())
                })
        );

        Some(ErrorCode::Success.to_ret())
    }

    pub fn listen(&self, ctx: InvokeContext) -> Option<Value> {
        let addr = ctx.extract_str(0, 1);
        let cb_target = ctx.args[2].get_i32().unwrap();
        let cb_data = ctx.args[3].get_i32().unwrap();

        let app = ctx.app.upgrade().unwrap();

        match app.check_permission(&AppPermission::TcpListenAny)
            .or_else(|_| app.check_permission(&AppPermission::TcpListen(addr.to_string()))) {
                Ok(_) => {},
                Err(_) => {
                    derror!(
                        logger!(&app.name),
                        "TcpListenAny or TcpListen({}) permission is required",
                        addr
                    );
                    return Some(ErrorCode::PermissionDenied.to_ret());
                }
            }

        let app_weak = ctx.app.clone();

        let saddr: SocketAddr = addr.parse().unwrap();
        let listener = tokio::net::TcpListener::bind(&saddr).unwrap();

        let streams = self.streams.clone();

        tokio::executor::current_thread::spawn(
            listener.incoming().for_each(move |stream| {
                let (rh, wh) = stream.split();
                let stream_id = streams.borrow_mut().insert((
                    Some(rh),
                    Some(wh)
                ));

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

        let conn = self.streams.borrow_mut()[stream_id].0.take().unwrap();
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

        Some(ErrorCode::Success.to_ret())
    }

    pub fn write(&self, ctx: InvokeContext) -> Option<Value> {
        let stream_id = ctx.args[0].get_i32().unwrap() as usize;
        let data = ctx.extract_bytes(1, 2);
        let cb_target = ctx.args[3].get_i32().unwrap();
        let cb_data = ctx.args[4].get_i32().unwrap();

        let conn = self.streams.borrow_mut()[stream_id].1.take().unwrap();
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

        Some(ErrorCode::Success.to_ret())
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
