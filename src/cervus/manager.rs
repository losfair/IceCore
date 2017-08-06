use std;
use std::sync::{atomic, mpsc};
use std::sync::Mutex;
use std::collections::HashMap;
use futures::sync::oneshot;
use cervus::engine;
use cervus::value_type::ValueType;
use logging;

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
struct ModuleInitConfig {
    ok: i8
}

impl ModuleInitConfig {
    fn new() -> ModuleInitConfig {
        ModuleInitConfig {
            ok: 0
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

    let mut modules: HashMap<String, engine::Module> = HashMap::new();

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
                            {
                                let ee = engine::ExecutionEngine::new(&m);
                                let initializer = engine::Function::new_null_handle(&m, "cervus_module_init", ValueType::Void, vec![]);

                                let initializer = ee.get_callable_0::<()>(&initializer);
                                initializer();
                            }

                            modules.insert(name, m);
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
