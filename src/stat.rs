use std::collections::{HashMap, VecDeque};
use std::sync::atomic;
use std::sync::RwLock;
use time;
use serde_json;
use serde_derive;

pub struct ServerStats {
    endpoint_hits: RwLock<HashMap<String, atomic::AtomicUsize>>
}

impl ServerStats {
    pub fn new() -> ServerStats {
        ServerStats {
            endpoint_hits: RwLock::new(HashMap::new())
        }
    }

    pub fn inc_endpoint_hit(&self, ep_path: String) {
        let need_insert = match self.endpoint_hits.read().unwrap().get(&ep_path) {
            Some(v) => {
                v.fetch_add(1, atomic::Ordering::SeqCst);
                false
            },
            None => {
                true
            }
        };
        if need_insert {
            self.endpoint_hits.write().unwrap().insert(ep_path, atomic::AtomicUsize::new(1));
        }
    }

    pub fn serialize(&self) -> serde_json::Value {
        let endpoint_hits = self.endpoint_hits.read().unwrap();
        let mut endpoint_hits_map = HashMap::new();

        endpoint_hits.iter().map(|(k, v)| {
            endpoint_hits_map.insert(k.clone(), v.load(atomic::Ordering::Relaxed));
            ()
        }).collect::<Vec<()>>();

        json!({
            "endpoint_hits": endpoint_hits_map
        })
    }
}
