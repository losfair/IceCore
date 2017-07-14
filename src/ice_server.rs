use std;
use std::error::Error;
use std::sync::{Arc, RwLock};
use hyper;
use hyper::server::{Http, Request, Response, Service};
use futures;
use futures::future::{FutureResult, Future};
use futures::{Async, Poll};
use delegates;
use router;

#[derive(Clone)]
pub struct IceServer {
    pub context: Arc<Context>
}

pub struct Context {
    pub router: RwLock<router::Router>
}

struct HttpService {
    context: Arc<Context>
}

impl IceServer {
    pub fn new() -> IceServer {
        return IceServer {
            context: Arc::new(Context {
                router: RwLock::new(router::Router::new())
            })
        }
    }

    pub fn listen_in_this_thread(&self, addr: &str) {
        let addr = addr.parse().unwrap();
        let ctx = self.context.clone();

        let server = Http::new().bind(&addr, move || Ok(HttpService {
            context: ctx.clone()
        })).unwrap();

        server.run().unwrap();
    }

    pub fn listen(&self, addr: &str) -> std::thread::JoinHandle<()> {
        let addr = addr.to_string();

        let target = self.clone();

        std::thread::spawn(move || target.listen_in_this_thread(addr.as_str()))
    }
}

impl Service for HttpService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<futures::Future<Error=hyper::Error, Item=hyper::Response>>;

    fn call(&self, req: Request) -> Self::Future {
        println!("{}", req.remote_addr().unwrap());

        Box::new(delegates::fire_handlers(self.context.clone(), req)
        .map(|resp| {
            Response::new().with_headers(resp.get_headers()).with_body(resp.get_body())
        }).map_err(|e| hyper::Error::from(std::io::Error::new(std::io::ErrorKind::Other, e))))
    }
}
