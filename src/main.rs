#![feature(nll)]

extern crate wasm_translator;
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

mod lssa;
mod container;
mod config;
mod server;

use std::io::Read;

use wasm_translator::wasm_core;
use config::Config;
use server::Server;

fn main() {
    let config_path = ::std::env::args().nth(1).unwrap();
    let config = load_config(&config_path);

    let server = Server::new(config);
    tokio::run(
        server.run_apps()
    );
}

fn load_config(path: &str) -> Config {
    let mut config_file = ::std::fs::File::open(path)
        .unwrap_or_else(|e| {
            panic!("Unable to open configuration file located at {}: {:?}", path, e)
        });

    let mut config_text = String::new();
    config_file.read_to_string(&mut config_text).unwrap();

    let config = serde_yaml::from_str(&config_text).unwrap_or_else(|e| {
        panic!("Unable to parse configuration: {:?}", e);
    });

    config
}
