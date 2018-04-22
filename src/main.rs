#![feature(nll)]

extern crate wasm_core;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate slab;
extern crate futures;
extern crate tokio;
extern crate ansi_term;
extern crate chrono;
extern crate tokio_io;
extern crate bincode;

#[macro_use]
mod logging;

#[macro_use]
mod lssa;

mod container;
mod config;
mod server;

use std::panic::catch_unwind;
use config::Config;
use server::Server;

fn main() {
    let config_path = ::std::env::args().nth(1).unwrap_or_else(|| {
        derror!(
            logger!("(main)"),
            "Expecting path to config file as the first command-line argument"
        );
        ::std::process::exit(1);
    });
    let config = match catch_unwind(|| Config::from_file(&config_path)) {
        Ok(v) => v,
        Err(_) => {
            derror!(
                logger!("(main)"),
                "Invalid config file"
            );
            ::std::process::exit(1);
        }
    };

    let server = Server::new(config);

    tokio::executor::current_thread::block_on_all(
        server.run_apps()
    ).unwrap();
}
