use std;
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex, RwLock};
use std::ops::Deref;
//use std::sync::atomic;
use uuid::Uuid;
use time;
use logging;
use session_storage::*;

pub struct MemoryStorage {
    inner: Arc<MemoryStorageImpl>
}

impl Deref for MemoryStorage {
    type Target = MemoryStorageImpl;
    fn deref(&self) -> &MemoryStorageImpl {
        &self.inner
    }
}

pub struct MemoryStorageImpl {
    sessions: RwLock<BTreeMap<String, Arc<MemorySession>>>,
    timeout_ms: u64,
    period_ms: u64
}

pub struct MemorySession {
    inner: Mutex<MemorySessionImpl>
}

struct MemorySessionImpl {
    id: String,
    //create_time: u64,
    last_active_time: u64,
    //generation: usize,
    pub data: HashMap<String, String>
}

impl SessionProvider for MemorySession {
    fn get_id(&self) -> String {
        self.inner.lock().unwrap().id.clone()
    }

    fn get(&self, key: &str) -> Option<String> {
        match self.inner.lock().unwrap().data.get(key) {
            Some(v) => Some(v.clone()),
            None => None
        }
    }

    fn set(&self, key: &str, value: &str) {
        self.inner.lock().unwrap().data.insert(key.to_string(), value.to_string());
    }
}

impl SessionStorageProvider for MemoryStorage {
    fn create_session(&self) -> Session {
        let id = Uuid::new_v4().to_string();
        let t = time::millis();

        let sess = Arc::new(MemorySession {
            inner: Mutex::new(MemorySessionImpl {
                id: id.clone(),
                //create_time: t,
                last_active_time: t,
                //generation: self.current_generation.load(atomic::Ordering::Relaxed),
                data: HashMap::new()
            })
        });

        self.sessions.write().unwrap().insert(id, sess.clone());
        sess.into()
    }

    fn get_session(&self, id: &str) -> Option<Session> {
        let sess = match self.sessions.read().unwrap().get(&id.to_string()) {
            Some(v) => v.clone(),
            None => return None
        };
        let t = time::millis();
        sess.inner.lock().unwrap().last_active_time = t;
        Some(sess.clone().into())
    }

    fn start(&self) {
        let target = self.inner.clone();
        std::thread::spawn(move || MemoryStorage::run_gc(target));
    }
}

impl MemoryStorage {
    pub fn new(timeout_ms: u64, period_ms: u64) -> MemoryStorage {
        MemoryStorage {
            inner: Arc::new(MemoryStorageImpl {
                sessions: RwLock::new(BTreeMap::new()),
                timeout_ms: timeout_ms,
                period_ms: period_ms
            })
        }
    }

    fn run_gc(target: Arc<MemoryStorageImpl>) {
        loop {
            target.gc(target.timeout_ms);
            std::thread::sleep(std::time::Duration::from_millis(target.period_ms));
        }
    }
}

impl MemoryStorageImpl {
    fn gc(&self, timeout_ms: u64) {
        let logger = logging::Logger::new("SessionStorage::gc");

        let mut to_remove: Vec<String> = Vec::new();
        let current_time = time::millis();

        {
            let sessions = self.sessions.read().unwrap();

            for (k, v) in sessions.iter() {
                if current_time - v.inner.lock().unwrap().last_active_time > timeout_ms {
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
}
