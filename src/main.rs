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

#[macro_use]
mod logging;

#[macro_use]
mod lssa;

mod container;
mod config;
mod server;

use std::io::Read;

use config::Config;
use server::Server;

use tokio::executor::current_thread::CurrentThread;

fn main() {
    let config_path = ::std::env::args().nth(1).unwrap();
    let config = Config::from_file(&config_path);

    let server = Server::new(config);

    tokio::run(server.run_apps());
}
