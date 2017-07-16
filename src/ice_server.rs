use std;
use std::error::Error;
use std::sync::{Arc, RwLock};
use hyper;
use hyper::server::{Http, Request, Response, Service};
use futures;
use futures::future::{FutureResult, Future};
use futures::{Async, Poll};
use futures::Stream;
use delegates;
use router;
use tokio_core;
use static_file;
use session_storage::SessionStorage;

#[derive(Clone)]
pub struct IceServer {
    pub prep: Arc<Preparation>
}

pub struct Preparation {
    pub router: Arc<RwLock<router::Router>>,
    pub static_dir: RwLock<Option<String>>
}

pub struct Context {
    pub router: Arc<RwLock<router::Router>>,
    pub static_dir: Option<String>,
    pub ev_loop_handle: tokio_core::reactor::Handle,
    pub static_file_worker: std::thread::JoinHandle<()>,
    pub static_file_worker_control_tx: std::sync::mpsc::Sender<static_file::WorkerControlMessage>,
    pub session_storage: Arc<SessionStorage>
}

struct HttpService {
    context: Arc<Context>
}

impl IceServer {
    pub fn new() -> IceServer {
        IceServer {
            prep: Arc::new(Preparation {
                router: Arc::new(RwLock::new(router::Router::new())),
                static_dir: RwLock::new(None)
            })
        }
    }

    pub fn listen_in_this_thread(&self, addr: &str) {
        let addr = addr.parse().unwrap();

        let mut ev_loop = tokio_core::reactor::Core::new().unwrap();

        let (control_tx, control_rx) = std::sync::mpsc::channel();
        let remote_handle = ev_loop.handle().remote().clone();

        let static_file_worker = std::thread::spawn(move || static_file::worker(remote_handle, control_rx));

        let mut session_storage = Arc::new(SessionStorage::new());

        let mut ctx = Arc::new(Context {
            router: self.prep.router.clone(),
            static_dir: self.prep.static_dir.read().unwrap().clone(),
            ev_loop_handle: ev_loop.handle(),
            static_file_worker: static_file_worker,
            static_file_worker_control_tx: control_tx,
            session_storage: session_storage.clone()
        });

        let _ = std::thread::spawn(move || session_storage.run_gc(600000, 10000));

        let this_handle = ev_loop.handle();

        let listener = tokio_core::net::TcpListener::bind(&addr, &this_handle).unwrap();

        let server = listener.incoming().for_each(|(sock, addr)| {
            let s = HttpService {
                context: ctx.clone()
            };
            Http::new().bind_connection(&this_handle, sock, addr, s);

            Ok(())
        });

        ev_loop.run(server).unwrap();
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
        .map_err(|e| hyper::Error::from(std::io::Error::new(std::io::ErrorKind::Other, e))))
    }
}
