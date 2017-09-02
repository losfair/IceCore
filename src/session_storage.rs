use std::sync::Arc;
use std::ops::Deref;
use futures::{future, Future};

pub struct SessionStorage {
    provider: Box<SessionStorageProvider + Send + Sync>
}

#[derive(Clone)]
pub struct Session {
    provider: Arc<SessionProvider + Send + Sync>
}

impl<T> From<Box<T>> for SessionStorage where T: SessionStorageProvider + Send + Sync + 'static {
    fn from(other: Box<T>) -> SessionStorage {
        SessionStorage {
            provider: other
        }
    }
}

impl Deref for SessionStorage {
    type Target = SessionStorageProvider + Send + Sync + 'static;
    fn deref(&self) -> &Self::Target {
        &*self.provider
    }
}

impl<T> From<Arc<T>> for Session where T: SessionProvider + Send + Sync + 'static {
    fn from(other: Arc<T>) -> Session {
        Session {
            provider: other
        }
    }
}

impl Deref for Session {
    type Target = SessionProvider + Send + Sync + 'static;
    fn deref(&self) -> &Self::Target {
        &*self.provider
    }
}

pub trait SessionStorageProvider {
    fn create_session(&self) -> Session;
    fn create_session_async(&self) -> Box<Future<Item = Session, Error = ()>> {
        Box::new(future::ok(self.create_session()))
    }
    fn get_session(&self, id: &str) -> Option<Session>;
    fn get_session_async(&self, id: &str) -> Box<Future<Item = Option<Session>, Error = ()>> {
        Box::new(future::ok(self.get_session(id)))
    }
    fn start(&self) {}
}

pub trait SessionProvider {
    fn get_id(&self) -> String;
    fn get(&self, key: &str) -> Option<String>;
    fn get_async(&self, key: &str) -> Box<Future<Item = Option<String>, Error = ()>> {
        Box::new(future::ok(self.get(key)))
    }
    fn set(&self, key: &str, value: &str);
    fn set_async(&self, key: &str, value: &str) -> Box<Future<Item = (), Error = ()>> {
        Box::new(future::ok(self.set(key, value)))
    }
    fn remove(&self, key: &str) {
        self.set(key, "");
    }
    fn remove_async(&self, key: &str) -> Box<Future<Item = (), Error = ()>> {
        Box::new(future::ok(self.remove(key)))
    }
}
