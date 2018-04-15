use futures::prelude::*;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct NextTick {
    started: bool,
    notify: Arc<AtomicBool>
}

impl Future for NextTick {
    type Item = ();
    type Error = !;

    fn poll(
        &mut self
    ) -> Result<Async<()>, !> {
        if self.notify.load(Ordering::Relaxed) == true {
            return Ok(Async::Ready(()));
        }

        if !self.started {
            self.started = true;
            let notify = self.notify.clone();
            let task = ::executor::current_task();

            ::schedule(move || {
                notify.store(true, Ordering::Relaxed);
                ::executor::run_once_next_tick(&task);
            });
        }

        Ok(Async::NotReady)
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
