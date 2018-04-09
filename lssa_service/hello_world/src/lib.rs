#[macro_use]
extern crate lssa_service;

use lssa_service::futures;
use lssa_service::futures::prelude::*;

fn fib(n: i32) -> i32 {
    if n == 1 || n == 2 {
        1
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

app_init!({
    println!("Hello world");
    let mut host = lssa_service::executor::Host::new();
    host.spawn(Box::new(
        lssa_service::utils::NextTick::new()
            .map(|_| {
                println!("Next tick!");
                lssa_service::utils::NextTick::new()
            })
            .flatten()
            .map(|_| {
                println!("Next tick 2!");
            })
            .map(|_| {
                println!("Callback!");
            })
    )).unwrap();
    host.spawn(Box::new(
        lssa_service::utils::TcpListener::new(
            "127.0.0.1:1111"
        ).for_each(|conn| {
            println!("Got connection");
            Ok(())
        }).map(|_| ())
    )).unwrap();
    println!("End of init");
    /*
    lssa_service::listen_tcp(
        "127.0.0.1:1111",
        |s| {
            let s2 = s.clone();
            s.write(
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".as_bytes(),
                |_| {
                    drop(s2);
                }
            );
        }
    ).unwrap();
    lssa_service::set_timeout(1000, || {
        eprintln!("Hello world 2");
    });
    lssa_service::schedule(|| {
        eprintln!("Hello world 3");
        let start_time = lssa_service::time();
        let result = fib(3);
        let end_time = lssa_service::time();
        eprintln!("fib(3) = {}, time = {} ms", result, end_time - start_time);
    });
    */
    0
});
