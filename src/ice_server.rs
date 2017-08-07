use std;
use std::collections::HashMap;
use std::sync::{Arc, Weak, RwLock, Mutex};
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
use time;

#[cfg(feature = "cervus")]
use cervus;

#[derive(Clone)]
pub struct IceServer {
    pub prep: Arc<Preparation>
}

/*
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

            let f = Function::new(&m, "test_function", ValueType::Int64, vec![ValueType::Int64, ValueType::Int64]);
            let bb = BasicBlock::new(&f, "test_block");
            let mut builder = Builder::new(&bb);

            let ret = builder.append(Action::IntAdd(f.get_param(0).unwrap(), f.get_param(1).unwrap()));
            builder.append(Action::Return(ret));

            let ee = ExecutionEngine::new(&m);
            let callable = ee.get_callable_2::<i64, i64, i64>(&f);

            let ret = unsafe {
                callable(5, 2)
            };
            if ret != 7 {
                panic!("Incorrect return value from jitted function: {}", ret);
            }
            
            let loop_count = 10000000;

            let start_time = time::millis();

            for i in 0..loop_count {
                unsafe {
                    callable(1, 2);
                }
            }

            let end_time = time::millis();
            logger.log(logging::Message::Info(format!("Time for adding {} times: {} ms", loop_count, end_time - start_time)));
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
*/

pub enum Hook {
    ContextInit(Arc<Context>)
}

#[cfg(feature = "cervus")]
struct ModuleData {
    config: Weak<cervus::manager::ModuleConfig>,
    mem: Option<Vec<u8>>
}

#[cfg(feature = "cervus")]
pub struct Modules {
    data: HashMap<String, ModuleData>
}

#[cfg(feature = "cervus")]
impl Modules {
    fn new() -> Modules {
        Modules {
            data: HashMap::new()
        }
    }

    pub fn update(&mut self, control_tx: std::sync::mpsc::Sender<cervus::manager::ControlMessage>) {
        let (result_tx, result_rx) = std::sync::mpsc::channel();

        control_tx.send(cervus::manager::ControlMessage {
            result_tx: cervus::manager::ResultChannel::Mpsc(result_tx),
            action: cervus::manager::ControlAction::GetModuleList
        }).unwrap();

        let result = result_rx.recv().unwrap();
        let module_list = match result {
            cervus::manager::ResultMessage::ModuleList(v) => v,
            _ => panic!("Unexpected result from Cervus manager")
        };
        
        for name in module_list {
            let (result_tx, result_rx) = std::sync::mpsc::channel();

            control_tx.send(cervus::manager::ControlMessage {
                result_tx: cervus::manager::ResultChannel::Mpsc(result_tx),
                action: cervus::manager::ControlAction::GetModuleConfig(name.clone())
            }).unwrap();

            let result = result_rx.recv().unwrap();
            let cfg = match result {
                cervus::manager::ResultMessage::ModuleConfig(v) => v,
                _ => panic!("Unexpected result from Cervus manager")
            };

            let mem = {
                let cfg = cfg.upgrade().unwrap();

                if cfg.server_context_mem_size == 0 {
                    None
                } else {
                    Some(vec![0; cfg.server_context_mem_size as usize])
                }
            };

            self.data.insert(name, ModuleData {
                config: cfg,
                mem: mem
            });
        }
    }

    pub fn run_hook(&self, hook: Hook) {
        match hook {
            Hook::ContextInit(ctx) => {
                for (_, m) in self.data.iter() {
                    let cfg = match m.config.upgrade() {
                        Some(v) => v,
                        None => {
                            continue;
                        }
                    };
                    let mem = match m.mem {
                        Some(ref v) => v.as_ptr() as *mut u8,
                        None => std::ptr::null_mut()
                    };
                    
                    match cfg.context_init_hook {
                        Some(f) => {
                            f(mem, &*ctx);
                        },
                        None => {
                            continue;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "cervus"))]
pub struct Modules {}

#[cfg(not(feature = "cervus"))]
impl Modules {
    fn new() -> Modules {
        Modules {}
    }

    pub fn update(&mut self, _: bool) {}

    pub fn run_hook(&self, _: Hook) {}
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
    pub custom_app_data: delegates::CustomAppData,
    pub cervus_modules: Arc<RwLock<Modules>>,
    #[cfg(feature = "cervus")] pub cervus_control_tx: Mutex<std::sync::mpsc::Sender<cervus::manager::ControlMessage>>,
    #[cfg(not(feature = "cervus"))] pub cervus_control_tx: Mutex<bool>
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
    pub custom_app_data: delegates::CustomAppData,
    pub cervus_modules: Arc<RwLock<Modules>>,
    #[cfg(feature = "cervus")] pub cervus_control_tx: Mutex<std::sync::mpsc::Sender<cervus::manager::ControlMessage>>,
    #[cfg(not(feature = "cervus"))] pub cervus_control_tx: Mutex<bool>
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

#[cfg(feature = "cervus")]
fn start_cervus_manager() -> std::sync::mpsc::Sender<cervus::manager::ControlMessage> {
    cervus::manager::start_manager()
}

#[cfg(not(feature = "cervus"))]
fn start_cervus_manager() -> bool {
    false
}

impl IceServer {
    pub fn new() -> IceServer {
        let cervus_control_tx = start_cervus_manager();

        let mut modules = Modules::new();
        modules.update(cervus_control_tx.clone());

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
                custom_app_data: delegates::CustomAppData::empty(),
                cervus_modules: Arc::new(RwLock::new(modules)),
                cervus_control_tx: Mutex::new(cervus_control_tx)
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
            custom_app_data: self.prep.custom_app_data.clone(),
            cervus_modules: self.prep.cervus_modules.clone(),
            cervus_control_tx: Mutex::new(self.prep.cervus_control_tx.lock().unwrap().clone())
        });

        let modules = self.prep.cervus_modules.clone();
        modules.read().unwrap().run_hook(Hook::ContextInit(ctx.clone()));

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
