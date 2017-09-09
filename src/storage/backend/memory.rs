use std;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use futures;
use futures::Future;
use storage::kv::{KVStorage, HashMapExt};
use storage::error::StorageError;
use trait_handle::TraitHandle;
use time;

pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<String, Value>>>,
    expire_info: Arc<Mutex<BinaryHeap<ExpireInfo>>>,
    expire_worker_stop_tx: Mutex<std::sync::mpsc::Sender<()>>,
    hash_map_ext: TraitHandle<HashMapExt + Send + Sync>
}

#[derive(Eq, PartialEq)]
struct ExpireInfo {
    key: String,
    expire_at: u64
}

impl Ord for ExpireInfo {
    fn cmp(&self, other: &ExpireInfo) -> Ordering {
        other.expire_at.cmp(&self.expire_at)
    }
}

impl PartialOrd for ExpireInfo {
    fn partial_cmp(&self, other: &ExpireInfo) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

enum Value {
    String(String),
    HashMap(HashMap<String, String>)
}

impl MemoryStorage {
    pub fn new() -> MemoryStorage {
        let data = Arc::new(RwLock::new(HashMap::new()));
        let expire_info = Arc::new(Mutex::new(BinaryHeap::new()));
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();

        {
            let data = data.clone();
            let expire_info = expire_info.clone();
            std::thread::spawn(move || MemoryStorage::expire_worker(
                data,
                expire_info,
                stop_rx
            ));
        }

        MemoryStorage {
            data: data.clone(),
            expire_info: expire_info,
            expire_worker_stop_tx: Mutex::new(stop_tx),
            hash_map_ext: (Box::new(MemoryStorageHashMapExt {
                data: data
            }) as Box<HashMapExt + Send + Sync>).into()
        }
    }

    fn expire_worker(
        data: Arc<RwLock<HashMap<String, Value>>>,
        expire_info: Arc<Mutex<BinaryHeap<ExpireInfo>>>,
        stop_rx: std::sync::mpsc::Receiver<()>
    ) {
        loop {
            if stop_rx.try_recv().is_ok() {
                return;
            }
            let t = time::millis();

            {
                let mut expire_info = expire_info.lock().unwrap();
                loop {
                    {
                        let top = expire_info.peek();
                        if top.is_none() {
                            break;
                        }
                        let top = top.unwrap();

                        if t > top.expire_at {
                            data.write().unwrap().remove(&top.key);
                        } else {
                            break;
                        }
                    }

                    expire_info.pop();
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(5000));
        }
    }
}

impl Drop for MemoryStorage {
    fn drop(&mut self) {
        self.expire_worker_stop_tx.lock().unwrap().send(()).unwrap();
    }
}

impl KVStorage for MemoryStorage {
    fn get(&self, k: &str) -> Box<Future<Item = Option<String>, Error = StorageError> + Send> {
        futures::future::ok(
            match self.data.read().unwrap().get(k) {
                Some(v) => match v {
                    &Value::String(ref v) => Some(v.clone()),
                    _ => None
                },
                None => None
            }
        ).boxed()
    }

    fn set(&self, k: &str, v: &str) -> Box<Future<Item = (), Error = StorageError> + Send> {
        self.data.write().unwrap().insert(
            k.to_string(),
            Value::String(v.to_string())
        );
        futures::future::ok(()).boxed()
    }

    fn remove(&self, k: &str) -> Box<Future<Item = (), Error = StorageError> + Send> {
        self.data.write().unwrap().remove(k);
        futures::future::ok(()).boxed()
    }

    fn expire_sec(&self, k: &str, t: u32) -> Box<Future<Item = (), Error = StorageError> + Send> {
        self.expire_info.lock().unwrap().push(
            ExpireInfo {
                key: k.to_string(),
                expire_at: time::millis() + t as u64 * 1000
            }
        );
        futures::future::ok(()).boxed()
    }
    
    fn get_hash_map_ext(&self) -> Option<&TraitHandle<HashMapExt + Send + Sync>> {
        Some(&self.hash_map_ext)
    }
}

pub struct MemoryStorageHashMapExt {
    data: Arc<RwLock<HashMap<String, Value>>>
}

impl HashMapExt for MemoryStorageHashMapExt {
    fn get(&self, k: &str, map_key: &str) -> Box<Future<Item = Option<String>, Error = StorageError> + Send> {
        futures::future::ok(match self.data.read().unwrap().get(k) {
            Some(v) => match v {
                &Value::HashMap(ref m) => match m.get(map_key) {
                    Some(v) => Some(v.clone()),
                    None => None
                },
                _ => None
            },
            None => None
        }).boxed()
    }

    fn set(&self, k: &str, map_key: &str, v: &str) -> Box<Future<Item = (), Error = StorageError> + Send> {
        let mut data = self.data.write().unwrap();

        if let Some(current) = data.get_mut(k) {
            if let &mut Value::HashMap(ref mut m) = current {
                m.insert(map_key.to_string(), v.to_string());
                return futures::future::ok(()).boxed();
            }
        }

        let mut m = HashMap::new();
        m.insert(
            map_key.to_string(),
            v.to_string()
        );

        data.insert(
            k.to_string(),
            Value::HashMap(m)
        );

        futures::future::ok(()).boxed()
    }

    fn remove(&self, k: &str, map_key: &str) -> Box<Future<Item = (), Error = StorageError> + Send> {
        let mut data = self.data.write().unwrap();

        if let Some(current) = data.get_mut(k) {
            if let &mut Value::HashMap(ref mut m) = current {
                m.remove(map_key);
            }
        }

        futures::future::ok(()).boxed()
    }
}
