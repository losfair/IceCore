use std;
use std::net::SocketAddr;
use std::rc::Rc;
use net2;
use super::HttpServer;
use futures;
use futures::Stream;
use tokio_core;
use hyper;

#[cfg(unix)]
use net2::unix::UnixTcpBuilderExt;

pub struct HttpServerExecutor {
    server: HttpServer
}

struct ExecutorContext {
    server: HttpServer
}

struct HttpService {
    ev_loop_handle: tokio_core::reactor::Handle,
    server: HttpServer
}

impl HttpServerExecutor {
    pub fn new(server: HttpServer) -> HttpServerExecutor {
        let mut ret = HttpServerExecutor {
            server: server
        };
        ret.start();
        ret
    }

    fn start(&self) {
        let server = self.server.clone();

        std::thread::spawn(move || {
            let ctx = ExecutorContext {
                server: server
            };
            ctx.run()
        });
    }
}

impl ExecutorContext {
    fn run(&self) {
        let mut ev_loop = tokio_core::reactor::Core::new().unwrap();
        let ev_loop_handle = ev_loop.handle();

        let cfg = self.server.config.clone();
        let listen_addr = match cfg.listen_addr {
            Some(ref v) => v,
            None => panic!("Listen address not set")
        };

        let raw_listener = start_listener(listen_addr);

        let protocol: hyper::server::Http = hyper::server::Http::new();
        let listener = tokio_core::net::TcpListener::from_listener(
            raw_listener,
            listen_addr,
            &ev_loop_handle
        ).unwrap();

        let server = listener.incoming().for_each(|(sock, addr)| {
            let s = HttpService {
                ev_loop_handle: ev_loop_handle.clone(),
                server: self.server.clone()
            };
            protocol.bind_connection(&ev_loop_handle, sock, addr, s);

            Ok(())
        });

        ev_loop.run(server).unwrap();
    }
}

impl hyper::server::Service for HttpService {
    type Request = hyper::server::Request;
    type Response = hyper::server::Response;
    type Error = hyper::Error;
    type Future = Box<futures::Future<Error=hyper::Error, Item=hyper::Response>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        Box::new(futures::future::ok(hyper::Response::new()))
    }
}

#[cfg(unix)]
fn start_listener(addr: &SocketAddr) -> std::net::TcpListener {
    net2::TcpBuilder::new_v4().unwrap()
        .reuse_address(true).unwrap()
        .reuse_port(true).unwrap()
        .bind(addr).unwrap()
        .listen(128).unwrap()
}

#[cfg(not(unix))]
fn start_listener(addr: &SocketAddr) -> std::net::TcpListener {
    net2::TcpBuilder::new_v4().unwrap()
        .reuse_address(true).unwrap()
        .bind(addr).unwrap()
        .listen(128).unwrap()
}
