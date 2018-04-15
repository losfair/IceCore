use futures::prelude::*;

use std::collections::VecDeque;
use std::cell::{RefCell, UnsafeCell};
use std::rc::Rc;

use error::IoResult;

/// A TCP listener that provides a stream of `TcpConnection`s.
pub struct TcpListener {
    notify: Rc<UnsafeCell<VecDeque<::raw::TcpStream>>>,
    listening: bool,
    addr: String
}

impl Stream for TcpListener {
    type Item = TcpConnection;
    type Error = !;

    fn poll(
        &mut self
    ) -> Result<Async<Option<TcpConnection>>, !> {
        if !self.listening {
            let task = ::executor::current_task();
            let notify = self.notify.clone();

            self.listening = true;

            ::raw::listen_tcp(&self.addr, move |s| {
                let notify = unsafe {
                    &mut *notify.get()
                };
                notify.push_back(s);
                ::executor::run_once_next_tick(&task);
            });
        }

        let notify = unsafe {
            &mut *self.notify.get()
        };
        match notify.pop_front() {
            Some(v) => Ok(Async::Ready(Some(TcpConnection {
                raw: v
            }))),
            None => Ok(Async::NotReady)
        }
    }
}

impl TcpListener {
    /// Listens on the specified address (in the format `ip:port`).
    ///
    /// The application must have `TcpListenAny` or `TcpListen(addr)` permissions.
    pub fn new(addr: &str) -> TcpListener {
        TcpListener {
            addr: addr.to_string(),
            listening: false,
            notify: Rc::new(
                UnsafeCell::new(VecDeque::new())
            )
        }
    }
}

/// A TCP connection that can be read from or written to.
#[derive(Clone)]
pub struct TcpConnection {
    raw: ::raw::TcpStream
}

impl TcpConnection {
    /// Connects to the specified address (in the format `ip:port`).
    ///
    /// The application must have `TcpConnectAny` or `TcpConnect(addr)` permissions.
    pub fn connect(addr: &str) -> ConnectFuture {
        ConnectFuture {
            started: false,
            addr: addr.into(),
            status: Rc::new(RefCell::new(None))
        }
    }

    /// Writes to the connection.
    ///
    /// The `Future` returned by this resolves after all bytes are written successfully
    /// or an error occurs.
    pub fn write(&self, data: Vec<u8>) -> WriteFuture {
        WriteFuture {
            started: false,
            stream: self.raw.clone(),
            data: data,
            status: Rc::new(RefCell::new(None))
        }
    }

    /// Reads from the connection.
    ///
    /// The `Future` returned by this resolves after something is read or an error occurs.
    pub fn read(&self, len: usize) -> ReadFuture {
        ReadFuture {
            started: false,
            max_len: len,
            stream: self.raw.clone(),
            status: Rc::new(RefCell::new(None))
        }
    }
}

/// A `Future` representing a pending `TcpConnection::connect` request.
pub struct ConnectFuture {
    started: bool,
    addr: String,
    status: Rc<RefCell<Option<IoResult<TcpConnection>>>>
}

impl Future for ConnectFuture {
    type Item = TcpConnection;
    type Error = ::error::Io;

    fn poll(
        &mut self
    ) -> Result<Async<TcpConnection>, ::error::Io> {
        if let Some(v) = self.status.borrow_mut().take() {
            return match v {
                Ok(v) => Ok(Async::Ready(v)),
                Err(e) => Err(e)
            };
        }

        if self.started {
            return Ok(Async::NotReady);
        }

        self.started = true;

        let status = self.status.clone();
        let task = ::executor::current_task();

        ::raw::connect_tcp(&self.addr, move |stream| {
            *status.borrow_mut() = Some(match stream {
                Ok(v) => Ok(TcpConnection { raw: v }),
                Err(e) => Err(e)
            });
            ::executor::run_once_next_tick(&task);
        });

         Ok(Async::NotReady)
    }
}

/// A `Future` representing a pending `TcpConnection::read` request.
pub struct ReadFuture {
    started: bool,
    max_len: usize,
    stream: ::raw::TcpStream,
    status: Rc<RefCell<Option<IoResult<Vec<u8>>>>>
}

impl Future for ReadFuture {
    type Item = Vec<u8>;
    type Error = ::error::Io;

    fn poll(
        &mut self
    ) -> Result<Async<Vec<u8>>, ::error::Io> {
        if let Some(v) = self.status.borrow_mut().take() {
            return match v {
                Ok(v) => Ok(Async::Ready(v)),
                Err(e) => Err(e)
            };
        }

        if self.started {
            return Ok(Async::NotReady);
        }

        self.started = true;

        let status = self.status.clone();
        let task = ::executor::current_task();
        let max_len = self.max_len;

        self.stream.read(self.max_len, move |buf| {
            *status.borrow_mut() = Some(match buf {
                Ok(buf) => {
                    let mut buffer: Vec<u8> = Vec::with_capacity(max_len);
                    unsafe {
                        buffer.set_len(max_len);
                    }
                    let real_len = buf.take(&mut buffer);
                    assert!(real_len <= max_len);
                    unsafe {
                        buffer.set_len(real_len);
                    }
                    Ok(buffer)
                },
                Err(e) => Err(e)
            });
            ::executor::run_once_next_tick(&task);
        });

        Ok(Async::NotReady)
    }
}

/// A `Future` representing a pending `TcpConnection::write` request.
pub struct WriteFuture {
    started: bool,
    stream: ::raw::TcpStream,
    data: Vec<u8>,
    status: Rc<RefCell<Option<IoResult<i32>>>>
}

impl Future for WriteFuture {
    type Item = usize;
    type Error = ::error::Io;

    fn poll(
        &mut self
    ) -> Result<Async<usize>, ::error::Io> {
        if let Some(v) = self.status.borrow_mut().take() {
            return match v {
                Ok(v) => Ok(Async::Ready(v as usize)),
                Err(e) => Err(e)
            };
        }

        if self.started {
            return Ok(Async::NotReady);
        }

        self.started = true;

        let status = self.status.clone();
        let task = ::executor::current_task();

        self.stream.write(&self.data, move |result| {
            *status.borrow_mut() = Some(result);
            ::executor::run_once_next_tick(&task);
        });

        Ok(Async::NotReady)
    }
}
