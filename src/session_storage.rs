use std;
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic;
use uuid::Uuid;
use time;

use logging;

pub struct SessionStorage {
    sessions: RwLock<BTreeMap<String, Arc<Mutex<Session>>>>,
    current_generation: atomic::AtomicUsize
}

pub struct Session {
    id: String,
    create_time: u64,
    last_active_time: u64,
    generation: usize,
    pub data: HashMap<String, String>
}

impl SessionStorage {
    pub fn new() -> SessionStorage {
        SessionStorage {
            sessions: RwLock::new(BTreeMap::new()),
            current_generation: atomic::AtomicUsize::new(0)
        }
    }

    pub fn create_session(&self) -> Arc<Mutex<Session>> {
        let id = Uuid::new_v4().to_string();
        let t = time::millis();

        let sess = Arc::new(Mutex::new(Session {
            id: id.clone(),
            create_time: t,
            last_active_time: t,
            generation: self.current_generation.load(atomic::Ordering::Relaxed),
            data: HashMap::new()
        }));

        self.sessions.write().unwrap().insert(id, sess.clone());
        sess
    }

    pub fn get_session(&self, id: &str) -> Option<Arc<Mutex<Session>>> {
        let sess = match self.sessions.read().unwrap().get(&id.to_string()) {
            Some(v) => v.clone(),
            None => return None
        };
        let t = time::millis();
        sess.lock().unwrap().last_active_time = t;
        Some(sess)
    }

    pub fn gc(&self, timeout_ms: u64) {
        let logger = logging::Logger::new("SessionStorage::gc");

        let mut to_remove: Vec<String> = Vec::new();
        let current_time = time::millis();

        {
            let sessions = self.sessions.read().unwrap();

            for (k, v) in sessions.iter() {
                if current_time - v.lock().unwrap().last_active_time > timeout_ms {
                    to_remove.push(k.clone());
                    //println!("Before removing {}: {} refs", k, Arc::strong_count(v));
                }
            }
        }

        if to_remove.len() == 0 {
            return;
        }

        logger.log(logging::Message::Info(format!("Removing {} sessions", to_remove.len())));

        {
            let mut sessions = self.sessions.write().unwrap();
            for k in to_remove.iter() {
                sessions.remove(k);
            }
        }
    }

    pub fn run_gc(&self, timeout_ms: u64, period_ms: u64) {
        loop {
            self.gc(timeout_ms);
            std::thread::sleep(std::time::Duration::from_millis(period_ms));
        }
    }
}

impl Session {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
}
