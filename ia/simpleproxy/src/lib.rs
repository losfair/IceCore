#![feature(proc_macro, generators)]

#[macro_use]
extern crate ia;

extern crate futures_await as futures;

use futures::prelude::*;
use ia::executor::Host;

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

app_init!({
    println!("Current time (in ms): {}", ia::time());

    Host::spawn(Box::new(
        run_proxy().or_else(|e| {
            eprintln!("{:?}", e);
            Ok(())
        })
    ));
    0
});
