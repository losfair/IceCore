use futures::prelude::*;
use futures::task::Context;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::collections::VecDeque;
use std::cell::UnsafeCell;
use std::ops::Deref;

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

pub struct TcpConnection {
    raw: ::TcpStream
}

impl TcpConnection {
    pub fn write(&self, data: Vec<u8>) -> WriteFuture {
        WriteFuture {
            started: false,
            stream: self.raw.clone(),
            data: data,
            notify: Arc::new(AtomicBool::new(false))
        }
    }
}

pub struct WriteFuture {
    started: bool,
    stream: ::TcpStream,
    data: Vec<u8>,
    notify: Arc<AtomicBool>
}

impl Future for WriteFuture {
    type Item = ();
    type Error = Never;

    fn poll(
        &mut self,
        cx: &mut Context
    ) -> Result<Async<()>, Never> {
        if self.notify.load(Ordering::Relaxed) == true {
            return Ok(Async::Ready(()));
        }

        if self.started {
            return Ok(Async::Pending);
        }

        self.started = true;

        let notify = self.notify.clone();
        let waker = cx.waker().clone();

        self.stream.write(&self.data, move |_| {
            notify.store(true, Ordering::Relaxed);
            waker.wake();
        });

        Ok(Async::Pending)
    }
}
