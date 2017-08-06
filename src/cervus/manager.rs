use std::sync::{atomic, mpsc};
use std::collections::HashMap;
use futures::sync::oneshot;
use cervus::engine;
use logging;

lazy_static! {
    static ref MANAGER_RUNNING: atomic::AtomicBool = atomic::AtomicBool::new(false);
}

pub struct ControlMessage {
    pub result_tx: oneshot::Sender<ResultMessage>,
    pub action: ControlAction
}

pub enum ControlAction {
    LoadBitcode(String, Vec<u8>)
}

pub enum ResultMessage {
    Ok,
    Err(String)
}

pub fn run_manager(control_rx: mpsc::Receiver<ControlMessage>) {
    if MANAGER_RUNNING.fetch_or(true, atomic::Ordering::SeqCst) {
        return;
    }

    let logger = logging::Logger::new("cervus::manager::run_manager");
    logger.log(logging::Message::Info("Cervus manager running".to_string()));

    let mut modules: HashMap<String, engine::Module> = HashMap::new();

    loop {
        let msg = control_rx.recv().unwrap();
        let ret = match msg.action {
            ControlAction::LoadBitcode(name, data) => {
                match engine::Module::from_bitcode(name.as_str(), data.as_slice()) {
                    Some(m) => {
                        modules.insert(name, m);
                        ResultMessage::Ok
                    },
                    None => ResultMessage::Err("Unable to load bitcode".to_string())
                }
            }
        };
        match msg.result_tx.send(ret) {
            Ok(_) => {},
            Err(_) => {
                logger.log(logging::Message::Warning("Unable to send result".to_string()));
            }
        }
    }
}
