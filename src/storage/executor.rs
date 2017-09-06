use std;
use tokio_core;
use futures;

lazy_static! {
    static ref GLOBAL_EVENT_LOOP: tokio_core::reactor::Remote = {
        let executor = Box::new(tokio_core::reactor::Core::new().unwrap());
        let handle = executor.remote();

        let executor: usize = Box::into_raw(executor) as usize;
        std::thread::spawn(move || {
            let executor = executor as *mut tokio_core::reactor::Core;
            let mut executor = unsafe {
                Box::from_raw(executor)
            };
            executor.run(futures::future::empty::<(), ()>()).unwrap();
        });

        handle
    };
}

pub fn get_event_loop() -> &'static tokio_core::reactor::Remote {
    &GLOBAL_EVENT_LOOP
}
