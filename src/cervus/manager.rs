use std;
use std::ops::Deref;
use std::sync::{atomic, mpsc};
use std::sync::Mutex;
use std::collections::HashMap;
use futures::sync::oneshot;
use cervus::engine;
use cervus::value_type::ValueType;
use logging;
use ice_server;
use glue;

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
    LoadBitcode(String, Vec<u8>)
}

pub enum ResultMessage {
    Ok,
    Err(String)
}

#[repr(C)]
struct ModuleConfig {
    ok: i8,
    server_init_hook: Option<extern fn (*const ice_server::IceServer)>,
    server_destroy_hook: Option<extern fn (*const ice_server::IceServer)>,
    request_hook: Option<extern fn (*const ice_server::Context, *const glue::request::Request)>,
    response_hook: Option<extern fn (*const ice_server::Context, *const glue::response::Response)>
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
            server_init_hook: None,
            server_destroy_hook: None,
            request_hook: None,
            response_hook: None
        }
    }
}

struct ModuleContext {
    ee: ModuleEE,
    config: ModuleConfig
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
                    logger.log(logging::Message::Info(format!("Loading bitcode: {}", name)));
                    match engine::Module::from_bitcode(name.as_str(), data.as_slice()) {
                        Some(m) => {
                            let ee = ModuleEE::from_module(m);

                            let initializer = engine::Function::new_null_handle(&ee.get_module(), "cervus_module_init", ValueType::Void, vec![ValueType::Pointer(Box::new(ValueType::Void))]);
                            let mut init_cfg = ModuleConfig::new();
                            let initializer = ee.get_callable_1::<(), *mut ModuleConfig>(&initializer);
                            initializer(&mut init_cfg);

                            if init_cfg.ok != 1 {
                                panic!("Module initialization failed");
                            }

                            modules.insert(name, ModuleContext {
                                ee: ee,
                                config: init_cfg
                            });
                            ResultMessage::Ok
                        },
                        None => ResultMessage::Err("Unable to load bitcode".to_string())
                    }
                }
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
