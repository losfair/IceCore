use std::collections::{HashMap, VecDeque};
use std::sync::atomic;
use std::sync::{Mutex, RwLock};
use serde_json;
use config;

pub struct ServerStats {
    endpoint_hits: RwLock<HashMap<String, atomic::AtomicUsize>>,
    endpoint_processing_times: RwLock<HashMap<String, Mutex<VecDeque<u64>>>>,
    custom: RwLock<HashMap<String, String>>
}

impl ServerStats {
    pub fn new() -> ServerStats {
        ServerStats {
            endpoint_hits: RwLock::new(HashMap::new()),
            endpoint_processing_times: RwLock::new(HashMap::new()),
            custom: RwLock::new(HashMap::new())
        }
    }

    pub fn inc_endpoint_hit(&self, ep_path: &str) {
        let need_insert = match self.endpoint_hits.read().unwrap().get(ep_path) {
            Some(v) => {
                v.fetch_add(1, atomic::Ordering::SeqCst);
                false
            },
            None => {
                true
            }
        };
        if need_insert {
            self.endpoint_hits.write().unwrap().insert(ep_path.to_string(), atomic::AtomicUsize::new(1));
        }
    }

    pub fn add_endpoint_processing_time(&self, ep_path: &str, t_micros: u64) {
        let need_insert = match self.endpoint_processing_times.read().unwrap().get(ep_path) {
            Some(v) => {
                let mut handle = v.lock().unwrap();
                handle.push_back(t_micros);
                while handle.len() > config::STAT_REQUEST_WINDOW_SIZE {
                    handle.pop_front();
                }
                false
            },
            None => true
        };
        if need_insert {
            let mut vd = VecDeque::new();
            vd.push_back(t_micros);
            self.endpoint_processing_times.write().unwrap().insert(ep_path.to_string(), Mutex::new(vd));
        }
    }

    pub fn set_custom(&self, k: String, v: String) {
        self.custom.write().unwrap().insert(k, v);
    }

    pub fn serialize(&self) -> serde_json::Value {
        let endpoint_hits = self.endpoint_hits.read().unwrap();
        let mut endpoint_hits_map = HashMap::new();

        endpoint_hits.iter().map(|(k, v)| {
            endpoint_hits_map.insert(k.clone(), v.load(atomic::Ordering::Relaxed));
            ()
        }).collect::<Vec<()>>();

        let mut ept_map = HashMap::new();
        self.endpoint_processing_times.read().unwrap().iter().map(|(k, v)| {
            let mut total: u64 = 0;
            let v_handle = v.lock().unwrap();
            for item in v_handle.iter() {
                total += *item;
            }
            ept_map.insert(k.clone(), total / v_handle.len() as u64);
            ()
        }).collect::<Vec<()>>();

        json!({
            "endpoint_hits": endpoint_hits_map,
            "endpoint_processing_times": ept_map,
            "custom": *self.custom.read().unwrap()
        })
    }
}
