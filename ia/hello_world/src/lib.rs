#![feature(proc_macro, generators)]

#[macro_use]
extern crate ia;

#[macro_use]
extern crate futures_await as futures;

use futures::prelude::*;

use ia::executor::Host;

fn fib(n: i32) -> i32 {
    if n == 1 || n == 2 {
        1
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

#[async]
fn handle_connection(incoming: ia::utils::TcpConnection) -> ia::error::IoResult<()> {
    #[async]
    fn handle_proxied_to_incoming(
        proxied: ia::utils::TcpConnection,
        incoming: ia::utils::TcpConnection
    ) -> ia::error::IoResult<()> {
        while let Ok(v) = await!(proxied.read(4096)) {
            if v.len() == 0 {
                break;
            }
            await!(incoming.write(v))?;
        }
        Ok(())
    }
    let proxied = await!(ia::utils::TcpConnection::connect("127.0.0.1:80"))?;
    Host::spawn(Box::new(
        handle_proxied_to_incoming(proxied.clone(), incoming.clone()).or_else(|e| {
            eprintln!("{:?}", e);
            Ok(())
        })
    ));
    while let Ok(v) = await!(incoming.read(4096)) {
        if v.len() == 0 {
            break;
        }
        await!(proxied.write(v))?;
    }

    Ok(())
}

#[async]
fn run_proxy() -> ia::error::IoResult<()> {
    let listener = ia::utils::TcpListener::new("127.0.0.1:1111");
    #[async]
    for incoming in listener {
        Host::spawn(Box::new(
            handle_connection(incoming).or_else(|e| {
                eprintln!("{:?}", e);
                Ok(())
            })
        ))
    }
    Ok(())
}

#[async]
fn tiny_http() -> ia::error::IoResult<()> {
    let listener = ia::utils::TcpListener::new("127.0.0.1:1112");
    #[async]
    for incoming in listener {
        await!(incoming.write(
            "HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n".as_bytes().to_vec()
        ))?;
    }

    Ok(())
}

app_init!({
    println!("Hello world! Time: {}", ia::time());

    Host::spawn(Box::new(
        tiny_http().or_else(|e| {
            eprintln!("{:?}", e);
            Ok(())
        })
    ));
    Host::spawn(Box::new(
        run_proxy().or_else(|e| {
            eprintln!("{:?}", e);
            Ok(())
        })
    ));
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
