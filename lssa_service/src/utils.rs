use futures::prelude::*;
use futures::task::Context;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct NextTick {
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

        let notify = self.notify.clone();
        let waker = cx.waker().clone();
        ::schedule(move || {
            notify.store(true, Ordering::Relaxed);
            waker.wake();
        });

        Ok(Async::Pending)
    }
}

impl NextTick {
    pub fn new() -> NextTick {
        NextTick {
            notify: Arc::new(AtomicBool::new(false))
        }
    }
}
