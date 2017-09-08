use std;
use tokio_core;
use futures;
use rand;
use num_cpus;
use logging;

lazy_static! {
    static ref GLOBAL_EVENT_LOOPS: Vec<tokio_core::reactor::Remote> = {
        let logger = logging::Logger::new("GLOBAL_EVENT_LOOPS");

        let mut ret = Vec::new();
        let n = num_cpus::get() * 2;

        logger.log(logging::Message::Info(format!("Creating {} reactors", n)));

        for _ in 0..n {
            ret.push(create_event_loop());
        }

        ret
    };
}

pub fn get_event_loop() -> &'static tokio_core::reactor::Remote {
    let id = rand::random::<usize>();
    &GLOBAL_EVENT_LOOPS[id % GLOBAL_EVENT_LOOPS.len()]
}

fn create_event_loop() -> tokio_core::reactor::Remote {
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
}
