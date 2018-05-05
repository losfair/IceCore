//! The runtime for writing applications for [Ice](https://github.com/losfair/IceCore),
//! an efficient, reliable and asynchronous platform for building modern backend applications
//! in WebAssembly.
//!
//! At a high level, `ia` (which stands for "Ice App") provides a few major components (based on
//! the underlying Ice Core engine):
//!
//! - Asynchronous TCP server and client
//! - File I/O
//! - Timer (not working for now due to an Ice bug)
//!
//! The asynchronous APIs are based on `futures`, while low-level callback-based APIs
//! are also provided.
//!
//! # Examples
//! A simple TCP proxy that forwards `127.0.0.1:1111` to `127.0.0.1:80`:
//! ```no_run
//! #![feature(proc_macro, generators)]
//!
//! #[macro_use]
//! extern crate ia;
//! extern crate futures_await as futures;
//! 
//! use futures::prelude::*;
//! use ia::net::{TcpListener, TcpConnection};
//! use ia::error::IoResult;
//! 
//! #[async]
//! fn handle_connection(incoming: TcpConnection) -> IoResult<()> {
//!     #[async]
//!     fn forward(from: TcpConnection, to: TcpConnection) -> IoResult<()> {
//!         while let Ok(v) = await!(from.read(4096)) {
//!             if v.len() == 0 {
//!                 break;
//!             }
//!             await!(to.write(v))?;
//!         }
//!         Ok(())
//!     }
//!     let proxied = await!(TcpConnection::connect("127.0.0.1:80"))?;
//!     ia::spawn(forward(proxied.clone(), incoming.clone()));
//!     await!(forward(incoming, proxied))?;
//! 
//!     Ok(())
//! }
//! 
//! #[async]
//! fn run_proxy() -> IoResult<()> {
//!     static LISTEN_ADDR: &'static str = "127.0.0.1:1111";
//!     let listener = TcpListener::new(LISTEN_ADDR);
//!     println!("Listening on {}", LISTEN_ADDR);
//! 
//!     #[async]
//!     for incoming in listener {
//!         ia::spawn(handle_connection(incoming));
//!     }
//! 
//!     Ok(())
//! }
//! 
//! app_init!({
//!     ia::spawn(run_proxy());
//!     0
//! });
//! 
//! ```
//!
//! See [simpleproxy](https://github.com/losfair/IceCore/tree/master/ia/examples/simpleproxy) for the
//! full code & project layout.

#![feature(fnbox)]
#![feature(never_type)]

pub extern crate futures;
pub extern crate cwa;

#[macro_use]
pub mod log;

pub mod raw;

pub mod executor;
pub mod utils;
pub mod error;
pub mod net;
pub mod fs;

pub use executor::spawn;
