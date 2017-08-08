use std;
use std::ops::Deref;
use std::sync::{atomic, mpsc};
use std::os::raw::{c_char, c_void};
use std::ffi::CStr;
use std::sync::{Arc, Weak, Mutex};
use std::collections::HashMap;
use futures::sync::oneshot;
use cervus::engine;
use cervus::value_type::ValueType;
use logging;
use ice_server;
use glue;
use delegates;

lazy_static! {
    static ref MANAGER_CONTROL_TX: Mutex<Option<mpsc::Sender<ControlMessage>>> = Mutex::new(None);
    static ref MANAGER_RUNNING: atomic::AtomicBool = atomic::AtomicBool::new(false);
}

pub enum ResultChannel {
    Mpsc(mpsc::Sender<ResultMessage>),
    Oneshot(oneshot::Sender<ResultMessage>)
}

pub struct ControlMessage {
    pub result_tx: ResultChannel,
    pub action: ControlAction
}

pub enum ControlAction {
    LoadBitcode(String, Vec<u8>),
    GetModuleConfig(String),
    GetModuleList
}

pub enum ResultMessage {
    Ok,
    Err(String),
    ModuleConfig(Weak<ModuleConfig>),
    ModuleList(Vec<String>)
}

#[repr(C)]
pub struct ModuleConfig {
    ok: i8,
    pub server_context_mem_size: u32,
    pub context_init_hook: Option<extern fn (*mut u8, delegates::ContextHandle)>,
    pub context_destroy_hook: Option<extern fn (*mut u8, delegates::ContextHandle)>,
    pub before_request_hook: Option<extern fn (*mut u8, *mut delegates::BasicRequestInfo)>,
    pub request_hook: Option<extern fn (*mut u8, *const glue::request::Request)>,
    pub response_hook: Option<extern fn (*mut u8, *const glue::response::Response)>
}

struct ModuleEE {
    _module_ref: *mut engine::Module,
    ee: engine::ExecutionEngine<'static>
}

impl ModuleEE {
    pub fn from_module(m: engine::Module) -> ModuleEE {
        let m = Box::new(m);
        let ee: engine::ExecutionEngine<'static> = unsafe {
            std::mem::transmute(engine::ExecutionEngine::new(&m))
        };
        ModuleEE {
            _module_ref: Box::into_raw(m),
            ee: ee
        }
    }
}

impl Drop for ModuleEE {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self._module_ref); }
    }
}

impl Deref for ModuleEE {
    type Target = engine::ExecutionEngine<'static>;
    fn deref(&self) -> &engine::ExecutionEngine<'static> {
        &self.ee
    }
}

impl ModuleConfig {
    fn new() -> ModuleConfig {
        ModuleConfig {
            ok: 0,
            server_context_mem_size: 0,
            context_init_hook: None,
            context_destroy_hook: None,
            before_request_hook: None,
            request_hook: None,
            response_hook: None
        }
    }
}

struct ModuleContext {
    ee: ModuleEE,
    resources: Box<ModuleResources>,
    config: Arc<ModuleConfig>
}

struct ModuleResources {
    logger: logging::Logger,
    allocs: HashMap<usize, Vec<u8>>
}

impl ModuleResources {
    fn new(name: &str) -> ModuleResources {
        ModuleResources {
            logger: logging::Logger::new(name),
            allocs: HashMap::new()
        }
    }
}

pub fn start_manager() -> mpsc::Sender<ControlMessage> {
    let mut control_tx_handle = MANAGER_CONTROL_TX.lock().unwrap();

    match *control_tx_handle {
        Some(ref v) => return v.clone(),
        None => {}
    }

    let (control_tx, control_rx) = mpsc::channel();
    std::thread::spawn(move || run_manager(control_rx));
    *control_tx_handle = Some(control_tx.clone());

    control_tx
}

fn run_manager(control_rx: mpsc::Receiver<ControlMessage>) {
    if MANAGER_RUNNING.fetch_or(true, atomic::Ordering::SeqCst) {
        panic!("Attempting to start Cervus manager again");
    }

    let logger = logging::Logger::new("cervus::manager::run_manager");
    logger.log(logging::Message::Info("Cervus manager started".to_string()));

    let mut modules: HashMap<String, ModuleContext> = HashMap::new();

    loop {
        let msg = control_rx.recv().unwrap();
        let ret = match msg.action {
            ControlAction::LoadBitcode(name, data) => {
                if !modules.get(&name).is_none() {
                    logger.log(logging::Message::Error(format!("Module {} already loaded", name)));
                    ResultMessage::Err("Module already exists".to_string())
                } else {
                    logger.log(logging::Message::Info(format!("Loading module: {}", name)));
                    match engine::Module::from_bitcode(name.as_str(), data.as_slice()) {
                        Some(m) => {
                            let mut module_res = Box::new(ModuleResources::new(&name));
                            let patch = engine::Module::new(format!("patch_{}", name).as_str());

                            patch.copy_data_layout_from(&m);

                            patch_module(&patch, &mut module_res);
                            m.link(patch);

                            let ee = ModuleEE::from_module(m);
                            ee.prepare();

                            ee.get_callable_1::<(), *const c_char>(&engine::Function::new_null_handle(&ee.get_module(), "cervus_log", ValueType::Void, vec![ValueType::Pointer(Box::new(ValueType::Int8))]));

                            let initializer = engine::Function::new_null_handle(&ee.get_module(), "cervus_module_init", ValueType::Void, vec![ValueType::Pointer(Box::new(ValueType::Void))]);
                            let mut init_cfg = ModuleConfig::new();
                            let initializer = ee.get_callable_1::<(), *mut ModuleConfig>(&initializer);
                            initializer(&mut init_cfg);

                            if init_cfg.ok != 1 {
                                panic!("Module initialization failed");
                            }

                            logger.log(logging::Message::Info(format!("Server context memory size: {}", init_cfg.server_context_mem_size)));

                            let module_ctx = ModuleContext {
                                ee: ee,
                                resources: module_res,
                                config: Arc::new(init_cfg)
                            };
                            modules.insert(name, module_ctx);

                            ResultMessage::Ok
                        },
                        None => ResultMessage::Err("Unable to load bitcode".to_string())
                    }
                }
            },
            ControlAction::GetModuleConfig(name) => {
                match modules.get(&name) {
                    Some(m) => ResultMessage::ModuleConfig(Arc::downgrade(&m.config)),
                    None => ResultMessage::Err("Module not found".to_string())
                }
            },
            ControlAction::GetModuleList => {
                ResultMessage::ModuleList(modules.iter().map(|(k, _)| k.to_owned()).collect())
            }
        };
        match msg.result_tx {
            ResultChannel::Mpsc(ch) => match ch.send(ret) {
                Ok(_) => {},
                Err(_) => {
                    logger.log(logging::Message::Warning("Unable to send result".to_string()));
                }
            },
            ResultChannel::Oneshot(ch) => match ch.send(ret) {
                Ok(_) => {},
                Err(_) => {
                    logger.log(logging::Message::Warning("Unable to send result".to_string()));
                }
            }
        }
    }
}

fn patch_module(m: &engine::Module, module_res: &mut ModuleResources) {
    add_logging_fn(m, module_res, "cervus_log", cervus_info);
    add_logging_fn(m, module_res, "cervus_info", cervus_info);
    add_logging_fn(m, module_res, "cervus_warning", cervus_warning);
    add_logging_fn(m, module_res, "cervus_error", cervus_error);
    add_logging_fn(m, module_res, "puts", cervus_info);
    add_malloc_fn(m, module_res, "malloc");
    add_free_fn(m, module_res, "free");
}

fn add_malloc_fn(m: &engine::Module, module_res: &mut ModuleResources, name: &str) {
    let malloc_fn = engine::Function::new(
        m,
        name,
        ValueType::Pointer(Box::new(ValueType::Void)),
        vec![ValueType::Int32]
    );
    let bb = engine::BasicBlock::new(&malloc_fn, "bb");
    let mut builder = engine::Builder::new(&bb);

    let module_res_addr = engine::Value::from(module_res as *mut ModuleResources as u64).const_int_to_ptr(
        ValueType::Pointer(Box::new(ValueType::Void))
    );

    let local_fn_addr = engine::Value::from(cervus_mm_malloc as *const c_void as u64).const_int_to_ptr(
        ValueType::Pointer(Box::new(ValueType::Function(
            Box::new(ValueType::Pointer(Box::new(ValueType::Void))),
            vec![
                ValueType::Pointer(Box::new(ValueType::Void)),
                ValueType::Int32
            ]
        )))
    );

    let ret = builder.append(engine::Action::Call(local_fn_addr, vec![module_res_addr, malloc_fn.get_param(0).unwrap()]));
    builder.append(engine::Action::Return(ret));
}

fn add_free_fn(m: &engine::Module, module_res: &mut ModuleResources, name: &str) {
    let free_fn = engine::Function::new(
        m,
        name,
        ValueType::Void,
        vec![ValueType::Pointer(Box::new(ValueType::Void))]
    );
    let bb = engine::BasicBlock::new(&free_fn, "bb");
    let mut builder = engine::Builder::new(&bb);

    let module_res_addr = engine::Value::from(module_res as *mut ModuleResources as u64).const_int_to_ptr(
        ValueType::Pointer(Box::new(ValueType::Void))
    );

    let local_fn_addr = engine::Value::from(cervus_mm_free as *const c_void as u64).const_int_to_ptr(
        ValueType::Pointer(Box::new(ValueType::Function(
            Box::new(ValueType::Void),
            vec![
                ValueType::Pointer(Box::new(ValueType::Void)),
                ValueType::Pointer(Box::new(ValueType::Void))
            ]
        )))
    );

    builder.append(engine::Action::Call(local_fn_addr, vec![module_res_addr, free_fn.get_param(0).unwrap()]));
    builder.append(engine::Action::ReturnVoid);
}

unsafe extern fn cervus_mm_malloc(resources: *mut ModuleResources, len: u32) -> *mut u8 {
    let resources = &mut *resources;

    //resources.logger.log(logging::Message::Info(format!("Allocating {} bytes", len)));
    
    let mut mem = vec![0 as u8; len as usize];

    let addr = mem.as_mut_ptr();
    resources.allocs.insert(addr as usize, mem);

    addr
}

unsafe extern fn cervus_mm_free(resources: *mut ModuleResources, addr: *mut c_void) {
    let resources = &mut *resources;
    //resources.logger.log(logging::Message::Info(format!("Freeing {}", addr as usize)));

    match resources.allocs.remove(&(addr as usize)) {
        Some(_) => {},
        None => panic!("Trying to free an invalid address")
    }
}

fn add_logging_fn(m: &engine::Module, module_res: &ModuleResources, name: &str, target: unsafe extern fn (*const logging::Logger, *const c_char)) {
    let log_fn = engine::Function::new(m, name, ValueType::Void, vec![ValueType::Pointer(Box::new(ValueType::Int8))]);
    let bb = engine::BasicBlock::new(&log_fn, "log_bb");
    let mut builder = engine::Builder::new(&bb);

    let logger_addr = &module_res.logger as *const logging::Logger;
    let logger_addr = engine::Value::from(logger_addr as u64).const_int_to_ptr(
        ValueType::Pointer(Box::new(ValueType::Void))
    );

    let local_fn_addr = engine::Value::from(target as *const c_void as u64).const_int_to_ptr(
        ValueType::Pointer(Box::new(ValueType::Function(Box::new(ValueType::Void), vec![
            ValueType::Pointer(Box::new(ValueType::Void)),
            ValueType::Pointer(Box::new(ValueType::Int8))
        ])))
    );

    builder.append(engine::Action::Call(local_fn_addr, vec![logger_addr, log_fn.get_param(0).unwrap()]));
    builder.append(engine::Action::ReturnVoid);
}

unsafe extern fn cervus_info(logger: *const logging::Logger, msg: *const c_char) {
    let logger = &*logger;
    logger.log(logging::Message::Info(CStr::from_ptr(msg).to_str().unwrap().to_string()));
}

unsafe extern fn cervus_warning(logger: *const logging::Logger, msg: *const c_char) {
    let logger = &*logger;
    logger.log(logging::Message::Warning(CStr::from_ptr(msg).to_str().unwrap().to_string()));
}

unsafe extern fn cervus_error(logger: *const logging::Logger, msg: *const c_char) {
    let logger = &*logger;
    logger.log(logging::Message::Error(CStr::from_ptr(msg).to_str().unwrap().to_string()));
}
