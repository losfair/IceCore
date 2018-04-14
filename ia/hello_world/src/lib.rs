#[macro_use]
extern crate ia;

use ia::futures;
use ia::futures::prelude::*;

fn fib(n: i32) -> i32 {
    if n == 1 || n == 2 {
        1
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

app_init!({
    println!("Hello world! Time: {}", ia::time());

    let mut host = ia::executor::Host::new();
    host.spawn(Box::new(
        ia::utils::NextTick::new()
            .map(|_| {
                println!("Next tick!");
                ia::utils::NextTick::new()
            })
            .flatten()
            .map(|_| {
                println!("Next tick 2!");
            })
            .map(|_| {
                eprintln!("Callback!");
            })
    )).unwrap();
    host.spawn(Box::new(
        ia::utils::TcpListener::new(
            "127.0.0.1:1111"
        ).for_each(|conn| {
            let conn2 = conn.clone();

            conn.read(4096)
                .and_then(move |data| {
                    conn2.write("HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n".as_bytes().to_vec())
                })
                .map(|_| ())
                .or_else(|e| {
                    eprintln!("{:?}", e);
                    Ok(())
                })
        }).map(|_| ())
    )).unwrap();
    println!("End of init");
    /*
    ia::listen_tcp(
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
    ia::set_timeout(1000, || {
        eprintln!("Hello world 2");
    });
    ia::schedule(|| {
        eprintln!("Hello world 3");
        let start_time = ia::time();
        let result = fib(3);
        let end_time = ia::time();
        eprintln!("fib(3) = {}, time = {} ms", result, end_time - start_time);
    });
    */
    0
});
