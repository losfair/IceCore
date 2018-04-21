#![feature(proc_macro, generators)]

#[macro_use]
extern crate ia;
extern crate futures_await as futures;

use futures::prelude::*;
use ia::net::{TcpListener, TcpConnection};
use ia::error::IoResult;

#[async]
fn handle_connection(incoming: TcpConnection) -> IoResult<()> {
    #[async]
    fn forward(from: TcpConnection, to: TcpConnection) -> IoResult<()> {
        while let Ok(v) = await!(from.read(4096)) {
            if v.len() == 0 {
                break;
            }
            await!(to.write(v))?;
        }
        Ok(())
    }
    let proxied = await!(TcpConnection::connect("127.0.0.1:80"))?;
    ia::spawn(forward(proxied.clone(), incoming.clone()));
    await!(forward(incoming, proxied))?;

    Ok(())
}

#[async]
fn run_proxy() -> IoResult<()> {
    let listen_addr = ia::cwa::env::get("LISTEN_ADDR")
        .unwrap_or_else(|| "127.0.0.1:1111".to_string());

    println!(
        "CommonWA spec version: {}.{}",
        ia::cwa::runtime::spec_major(),
        ia::cwa::runtime::spec_minor()
    );
    println!("Runtime: {}", ia::cwa::runtime::name());

    println!("Listening on {}", listen_addr);
    let listener = TcpListener::new(&listen_addr);

    #[async]
    for incoming in listener {
        ia::spawn(handle_connection(incoming));
    }

    Ok(())
}

app_init!({
    ia::spawn(run_proxy());
    0
});
