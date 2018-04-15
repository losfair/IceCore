#![feature(proc_macro, generators)]

#[macro_use]
extern crate ia;
extern crate futures_await as futures;

use std::io::Read;
use ia::error::IoResult;
use futures::prelude::*;

#[async]
fn handle_connection(conn: ia::net::TcpConnection) -> IoResult<()> {
    let _headers = await!(conn.read(4096))?;
    let mut cpuinfo_file = ia::fs::File::open("/proc/cpuinfo", "r")?;

    let mut content = Vec::new();
    cpuinfo_file.read_to_end(&mut content).unwrap();

    await!(conn.write(content))?;

    Ok(())
}

app_init!({
    let listener = ia::net::TcpListener::new("127.0.0.1:2231");
    ia::spawn(listener
        .for_each(|conn| {
            handle_connection(conn).or_else(|e| {
                eprintln!("{:?}", e);
                Ok(())
            })
        })
        .map(|_| ())
    );
    0
});
