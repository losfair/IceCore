use std::ops::Deref;
use futures::{future, Future};
use storage::error::StorageError;
use trait_handle::TraitHandle;
pub mod api;

pub trait KVStorage {
    fn get(&self, k: &str) -> Box<Future<Item = Option<String>, Error = StorageError> + Send>;
    fn set(&self, k: &str, v: &str) -> Box<Future<Item = (), Error = StorageError> + Send>;
    fn remove(&self, k: &str) -> Box<Future<Item = (), Error = StorageError> + Send>;
    fn expire_sec(&self, k: &str, t: u32) -> Box<Future<Item = (), Error = StorageError> + Send>;
    fn get_hash_map_ext<'a>(&'a self) -> Option<&'a TraitHandle<HashMapExt + Send + Sync>> {
        None
    }
}

pub trait HashMapExt {
    fn get(&self, k: &str, map_key: &str) -> Box<Future<Item = Option<String>, Error = StorageError> + Send>;
    fn set(&self, k: &str, map_key: &str, v: &str) -> Box<Future<Item = (), Error = StorageError> + Send>;
    fn remove(&self, k: &str, map_key: &str) -> Box<Future<Item = (), Error = StorageError> + Send>;
}
