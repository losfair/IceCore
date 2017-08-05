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

#[cfg(feature = "cervus")]
use cervus;

#[derive(Clone)]
pub struct IceServer {
    pub prep: Arc<Preparation>
}

#[cfg(feature = "cervus")]
pub struct CervusContext {
    module: cervus::engine::Module
}

#[cfg(feature = "cervus")]
impl CervusContext {
    pub fn new() -> CervusContext {
        let logger = logging::Logger::new("CervusContext::new");
        logger.log(logging::Message::Info("Testing Cervus".to_string()));

        let m = cervus::engine::Module::new("default");
        {
            use cervus::engine::*;
            use cervus::value_type::*;

            let f = Function::new(&m, "test_function", ValueType::Int32, vec![ValueType::Int32, ValueType::Int32]);
            let bb = BasicBlock::new(&f, "test_block");
            let mut builder = Builder::new(&bb);

            let ret = builder.append(Action::IntAdd(f.get_param(0).unwrap(), f.get_param(1).unwrap()));
            builder.append(Action::Return(ret));

            let ee = ExecutionEngine::new(&m);
            let ret = unsafe {
                ee.run(&f, vec![
                    GenericValue::from(5 as i32),
                    GenericValue::from(2 as i32)
                ])
            };
            let ret: i32 = ret.into();
            if ret != 7 {
                panic!("Incorrect return value from jitted function: {}", ret);
            }
        }

        logger.log(logging::Message::Info("OK".to_string()));
        CervusContext {
            module: m
        }
    }
}

#[cfg(not(feature = "cervus"))]
pub struct CervusContext {}

#[cfg(not(feature = "cervus"))]
impl CervusContext {
    pub fn new() -> CervusContext {
        CervusContext {}
    }
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
    pub async_endpoint_cb: Mutex<Option<extern fn (i32, *mut delegates::CallInfo)>>,
    pub custom_app_data: delegates::CustomAppData
}

pub struct Context {
    pub ev_loop_remote: tokio_core::reactor::Remote,
    pub router: router::Router,
    pub static_dir: Option<String>,
    pub session_cookie_name: String,
    pub session_storage: Arc<SessionStorage>,
    pub templates: Arc<TemplateStorage>,
    pub max_request_body_size: u32,
    pub log_requests: bool,
    pub stats: stat::ServerStats,
    pub max_cache_size: u32,
    pub endpoint_timeout_ms: u64,
    pub custom_app_data: delegates::CustomAppData
}

pub struct LocalContext {
    pub ev_loop_handle: tokio_core::reactor::Handle,
    pub static_file_worker_control_tx: std::sync::mpsc::Sender<static_file::WorkerControlMessage>,
    pub async_endpoint_cb: extern fn (i32, *mut delegates::CallInfo),
    pub cervus_context: CervusContext
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
                endpoint_timeout_ms: Mutex::new(config::DEFAULT_ENDPOINT_TIMEOUT_MS),
                custom_app_data: delegates::CustomAppData::empty()
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
            ev_loop_remote: remote_handle.clone(),
            router: self.prep.router.lock().unwrap().clone(),
            static_dir: self.prep.static_dir.read().unwrap().clone(),
            session_cookie_name: self.prep.session_cookie_name.lock().unwrap().clone(),
            session_storage: session_storage.clone(),
            templates: self.prep.templates.clone(),
            max_request_body_size: *self.prep.max_request_body_size.lock().unwrap(),
            log_requests: *self.prep.log_requests.lock().unwrap(),
            stats: stat::ServerStats::new(),
            max_cache_size: config::DEFAULT_MAX_CACHE_SIZE,
            endpoint_timeout_ms: *self.prep.endpoint_timeout_ms.lock().unwrap(),
            custom_app_data: self.prep.custom_app_data.clone()
        });

        let local_ctx = Rc::new(LocalContext {
            ev_loop_handle: ev_loop.handle(),
            static_file_worker_control_tx: control_tx,
            async_endpoint_cb: self.prep.async_endpoint_cb.lock().unwrap().clone().unwrap(),
            cervus_context: CervusContext::new()
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
