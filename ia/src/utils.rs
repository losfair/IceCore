use futures::prelude::*;
use futures::task::Context;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::collections::VecDeque;
use std::cell::{RefCell, UnsafeCell};
use std::ops::Deref;
use std::rc::Rc;

use error::IoResult;

pub struct NextTick {
    started: bool,
    notify: Arc<AtomicBool>
}

impl Future for NextTick {
    type Item = ();
    type Error = Never;

    fn poll(
        &mut self,
        cx: &mut Context
    ) -> Result<Async<()>, Never> {
        if self.notify.load(Ordering::Relaxed) == true {
            return Ok(Async::Ready(()));
        }

        if !self.started {
            self.started = true;
            let notify = self.notify.clone();
            let waker = cx.waker().clone();
            ::schedule(move || {
                notify.store(true, Ordering::Relaxed);
                waker.wake();
            });
        }

        Ok(Async::Pending)
    }
}

impl NextTick {
    pub fn new() -> NextTick {
        NextTick {
            started: false,
            notify: Arc::new(AtomicBool::new(false))
        }
    }
}

pub struct TcpListener {
    notify: Arc<UnsafeAssertSendSync<UnsafeCell<VecDeque<::TcpStream>>>>,
    listening: bool,
    addr: String
}

struct UnsafeAssertSendSync<T: ?Sized>(pub T);
unsafe impl<T: ?Sized> Send for UnsafeAssertSendSync<T> {}
unsafe impl<T: ?Sized> Sync for UnsafeAssertSendSync<T> {}

impl<T: ?Sized> Deref for UnsafeAssertSendSync<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl Stream for TcpListener {
    type Item = TcpConnection;
    type Error = Never;

    fn poll_next(
        &mut self,
        cx: &mut Context
    ) -> Result<Async<Option<TcpConnection>>, Never> {
        if !self.listening {
            let waker = cx.waker().clone();
            let notify = self.notify.clone();

            self.listening = true;

            ::listen_tcp(&self.addr, move |s| {
                let notify = unsafe {
                    &mut *notify.get()
                };
                notify.push_back(s);
                waker.wake();
            });
        }

        let notify = unsafe {
            &mut *self.notify.get()
        };
        match notify.pop_front() {
            Some(v) => Ok(Async::Ready(Some(TcpConnection {
                raw: v
            }))),
            None => Ok(Async::Pending)
        }
    }
}

impl TcpListener {
    pub fn new(addr: &str) -> TcpListener {
        TcpListener {
            addr: addr.to_string(),
            listening: false,
            notify: Arc::new(
                UnsafeAssertSendSync(UnsafeCell::new(VecDeque::new()))
            )
        }
    }
}

#[derive(Clone)]
pub struct TcpConnection {
    raw: ::TcpStream
}

impl TcpConnection {
    pub fn write(&self, data: Vec<u8>) -> WriteFuture {
        WriteFuture {
            started: false,
            stream: self.raw.clone(),
            data: data,
            status: Rc::new(RefCell::new(None))
        }
    }

    pub fn read(&self, len: usize) -> ReadFuture {
        ReadFuture {
            started: false,
            max_len: len,
            stream: self.raw.clone(),
            status: Rc::new(RefCell::new(None))
        }
    }
}

pub struct ReadFuture {
    started: bool,
    max_len: usize,
    stream: ::TcpStream,
    status: Rc<RefCell<Option<IoResult<Vec<u8>>>>>
}

// WebAssembly is single threaded (at least for now).
unsafe impl Send for ReadFuture {}

impl Future for ReadFuture {
    type Item = Vec<u8>;
    type Error = ::error::Io;

    fn poll(
        &mut self,
        cx: &mut Context
    ) -> Result<Async<Vec<u8>>, ::error::Io> {
        if let Some(v) = self.status.borrow_mut().take() {
            return match v {
                Ok(v) => Ok(Async::Ready(v)),
                Err(e) => Err(e)
            };
        }

        if self.started {
            return Ok(Async::Pending);
        }

        self.started = true;

        let status = self.status.clone();
        let waker = cx.waker().clone();
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
            waker.wake();
        });

        Ok(Async::Pending)
    }
}

pub struct WriteFuture {
    started: bool,
    stream: ::TcpStream,
    data: Vec<u8>,
    status: Rc<RefCell<Option<IoResult<i32>>>>
}

// WebAssembly is single threaded (at least for now).
unsafe impl Send for WriteFuture {}

impl Future for WriteFuture {
    type Item = usize;
    type Error = ::error::Io;

    fn poll(
        &mut self,
        cx: &mut Context
    ) -> Result<Async<usize>, ::error::Io> {
        if let Some(v) = self.status.borrow_mut().take() {
            return match v {
                Ok(v) => Ok(Async::Ready(v as usize)),
                Err(e) => Err(e)
            };
        }

        if self.started {
            return Ok(Async::Pending);
        }

        self.started = true;

        let status = self.status.clone();
        let waker = cx.waker().clone();

        self.stream.write(&self.data, move |result| {
            *status.borrow_mut() = Some(result);
            waker.wake();
        });

        Ok(Async::Pending)
    }
}
