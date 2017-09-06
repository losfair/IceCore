use futures::{future, Future};
use storage::error::StorageError;
pub mod api;

pub trait KVStorage {
    fn get(&self, k: &str) -> Box<Future<Item = Option<String>, Error = StorageError>>;
    fn set(&self, k: &str, v: &str) -> Box<Future<Item = (), Error = StorageError>>;
    fn remove(&self, k: &str) -> Box<Future<Item = (), Error = StorageError>>;
}
