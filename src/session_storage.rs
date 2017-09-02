use std::sync::Arc;
use std::ops::Deref;

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
    fn get_session(&self, id: &str) -> Option<Session>;
    fn start(&self) {}
}

pub trait SessionProvider {
    fn get_id(&self) -> String;
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str);
    fn remove(&self, key: &str) {
        self.set(key, "");
    }
}
