use std;
use std::fmt::Debug;
use std::sync::Arc;
use std::ops::Deref;
use futures::{future, Future};
use storage::kv::{KVStorage, HashMapExt};
use uuid::Uuid;

pub struct SessionStorage {
    storage: Arc<KVStorage + Send + Sync>,
    session_timeout_ms: u64
}

#[derive(Clone)]
pub struct Session {
    key: String,
    id: String,
    storage: Arc<KVStorage + Send + Sync>
}

impl Debug for Session {
    fn fmt(&self, _: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}

impl SessionStorage {
    pub fn new(storage: Arc<KVStorage + Send + Sync>, session_timeout_ms: u64) -> SessionStorage {
        SessionStorage {
            storage: storage,
            session_timeout_ms: session_timeout_ms
        }
    }

    pub fn create_session_async(&self) -> Box<Future<Item = Session, Error = ()>> {
        let id = Uuid::new_v4().to_string();
        let key = get_session_key_by_id(id.as_str());

        let sess = Session {
            key: key.clone(),
            id: id,
            storage: self.storage.clone()
        };

        let storage = self.storage.clone();
        let timeout_sec = (self.session_timeout_ms / 1000) as u32;

        let hm_ext = self.storage.get_hash_map_ext().unwrap();

        Box::new(hm_ext.set(key.as_str(), "_core_init", "")
            .map(move |_| {
                if timeout_sec > 0 {
                    storage.expire_sec(key.as_str(), timeout_sec)
                } else {
                    Box::new(future::ok(()))
                }
            })
            .flatten()
            .map(move |_| sess)
            .map_err(move |_| ())
        )
    }

    pub fn get_session_async(&self, id: &str) -> Box<Future<Item = Option<Session>, Error = ()>> {
        let key = get_session_key_by_id(id);
        let id = id.to_string();
        let storage = self.storage.clone();

        let hm_ext = self.storage.get_hash_map_ext().unwrap();

        Box::new(hm_ext.get(key.as_str(), "_core_init").map(move |v| {
            match v {
                Some(v) => Some(Session {
                    key: key,
                    id: id,
                    storage: storage
                }),
                None => None
            }
        }).map_err(|e| ()))
    }

    pub fn start(&self) {}
}

impl Session {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub fn get_async(&self, map_key: &str) -> Box<Future<Item = Option<String>, Error = ()>> {
        let hm_ext = self.storage.get_hash_map_ext().unwrap();

        Box::new(hm_ext.get(self.key.as_str(), map_key).map_err(|e| ()))
    }

    pub fn set_async(&self, map_key: &str, value: &str) -> Box<Future<Item = (), Error = ()>> {
        let hm_ext = self.storage.get_hash_map_ext().unwrap();

        Box::new(hm_ext.set(self.key.as_str(), map_key, value).map_err(|e| ()))
    }

    pub fn remove_async(&self, map_key: &str) -> Box<Future<Item = (), Error = ()>> {
        let hm_ext = self.storage.get_hash_map_ext().unwrap();

        Box::new(hm_ext.remove(self.key.as_str(), map_key).map_err(|e| ()))
    }
}

fn get_session_key_by_id(id: &str) -> String {
    "ice-session-".to_string() + id
}
