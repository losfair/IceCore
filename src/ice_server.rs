use std;
use std::sync::{Arc, RwLock, Mutex};
use std::rc::Rc;
use hyper;
use hyper::server::{Http, Request, Response, Service};
use futures;
use futures::future::Future;
use futures::Stream;
use delegates;
use router;
use tokio_core;
use static_file;
use logging;
use session_storage::SessionStorage;
use config;
use template::TemplateStorage;
use stat;

#[derive(Clone)]
pub struct IceServer {
    pub prep: Arc<Preparation>
}

pub struct Preparation {
    pub router: Arc<Mutex<router::Router>>,
    pub static_dir: RwLock<Option<String>>,
    pub session_cookie_name: Mutex<String>,
    pub session_timeout_ms: RwLock<u64>,
    pub templates: Arc<TemplateStorage>,
    pub max_request_body_size: Mutex<u32>,
    pub log_requests: Mutex<bool>,
    pub endpoint_timeout_ms: Mutex<u64>,
    pub async_endpoint_cb: Mutex<Option<extern fn (i32, *mut delegates::CallInfo)>>
}

pub struct Context {
    pub router: router::Router,
    pub static_dir: Option<String>,
    pub session_cookie_name: String,
    pub session_storage: Arc<SessionStorage>,
    pub templates: Arc<TemplateStorage>,
    pub max_request_body_size: u32,
    pub log_requests: bool,
    pub stats: stat::ServerStats,
    pub max_cache_size: u32,
    pub endpoint_timeout_ms: u64
}

pub struct LocalContext {
    pub ev_loop_handle: tokio_core::reactor::Handle,
    pub static_file_worker_control_tx: std::sync::mpsc::Sender<static_file::WorkerControlMessage>,
    pub async_endpoint_cb: extern fn (i32, *mut delegates::CallInfo)
}

struct HttpService {
    context: Arc<Context>,
    local_context: Rc<LocalContext>
}

impl IceServer {
    pub fn new() -> IceServer {
        IceServer {
            prep: Arc::new(Preparation {
                router: Arc::new(Mutex::new(router::Router::new())),
                static_dir: RwLock::new(None),
                session_cookie_name: Mutex::new(config::DEFAULT_SESSION_COOKIE_NAME.to_string()),
                session_timeout_ms: RwLock::new(600000),
                templates: Arc::new(TemplateStorage::new()),
                max_request_body_size: Mutex::new(config::DEFAULT_MAX_REQUEST_BODY_SIZE),
                log_requests: Mutex::new(true),
                async_endpoint_cb: Mutex::new(None),
                endpoint_timeout_ms: Mutex::new(config::DEFAULT_ENDPOINT_TIMEOUT_MS)
            })
        }
    }

    pub fn listen_in_this_thread(&self, addr: &str) {
        let logger = logging::Logger::new("IceServer::listen_in_this_thread");

        let addr = addr.parse().unwrap();

        let mut ev_loop = tokio_core::reactor::Core::new().unwrap();

        let (control_tx, control_rx) = std::sync::mpsc::channel();
        let remote_handle = ev_loop.handle().remote().clone();

        let session_storage = Arc::new(SessionStorage::new());

        let ctx = Arc::new(Context {
            router: self.prep.router.lock().unwrap().clone(),
            static_dir: self.prep.static_dir.read().unwrap().clone(),
            session_cookie_name: self.prep.session_cookie_name.lock().unwrap().clone(),
            session_storage: session_storage.clone(),
            templates: self.prep.templates.clone(),
            max_request_body_size: *self.prep.max_request_body_size.lock().unwrap(),
            log_requests: *self.prep.log_requests.lock().unwrap(),
            stats: stat::ServerStats::new(),
            max_cache_size: config::DEFAULT_MAX_CACHE_SIZE,
            endpoint_timeout_ms: *self.prep.endpoint_timeout_ms.lock().unwrap()
        });

        let local_ctx = Rc::new(LocalContext {
            ev_loop_handle: ev_loop.handle(),
            static_file_worker_control_tx: control_tx,
            async_endpoint_cb: self.prep.async_endpoint_cb.lock().unwrap().clone().unwrap()
        });

        let ctx_cloned = ctx.clone();
        let _ = std::thread::spawn(move || static_file::worker(ctx_cloned, remote_handle, control_rx));

        let session_timeout_ms = *self.prep.session_timeout_ms.read().unwrap();
        let _ = std::thread::spawn(move || session_storage.run_gc(session_timeout_ms, config::SESSION_GC_PERIOD_MS));

        let this_handle = ev_loop.handle();

        let listener = tokio_core::net::TcpListener::bind(&addr, &this_handle).unwrap();

        let server = listener.incoming().for_each(|(sock, addr)| {
            let s = HttpService {
                context: ctx.clone(),
                local_context: local_ctx.clone()
            };
            Http::new().bind_connection(&this_handle, sock, addr, s);

            Ok(())
        });

        logger.log(logging::Message::Info(format!("Ice Server v{} listening at {}", env!("CARGO_PKG_VERSION"), addr)));

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
        Box::new(delegates::fire_handlers(self.context.clone(), self.local_context.clone(), req)
        .map_err(|e| hyper::Error::from(std::io::Error::new(std::io::ErrorKind::Other, e))))
    }
}
