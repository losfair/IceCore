mod executor;
mod config;
mod path_utils;
mod router;
mod endpoint_context;
pub mod api;

use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::ops::Deref;
use http::server::executor::HttpServerExecutor;
use http::server::config::HttpServerConfig;
use http::server::router::HttpServerRoutingTable;


#[derive(Clone)]
pub struct HttpServer {
    inner: Arc<HttpServerImpl>
}

pub struct HttpServerImpl {
    config: HttpServerConfig,
    state: Mutex<HttpServerState>,
    routes: RwLock<HttpServerRoutingTable>
}

#[derive(Default)]
pub struct HttpServerState {
    started: bool
}

pub struct HttpServerExecutionContext {
    _executors: Vec<HttpServerExecutor>
}

impl HttpServer {
    pub fn new(config: HttpServerConfig) -> HttpServer {
        HttpServer {
            inner: Arc::new(HttpServerImpl {
                config: config,
                state: Mutex::new(HttpServerState::default()),
                routes: RwLock::new(HttpServerRoutingTable::new())
            })
        }
    }

    pub fn start(&self) -> Option<HttpServerExecutionContext> {
        let mut state = self.state.lock().unwrap();
        if state.started {
            return None;
        }

        let mut executors = Vec::with_capacity(self.config.num_executors);

        for _ in 0..self.config.num_executors {
            executors.push(HttpServerExecutor::new(self.clone()));
        }

        state.started = true;

        Some(HttpServerExecutionContext {
            _executors: executors
        })
    }

    pub fn get_routing_table<'a>(&'a self) -> RwLockReadGuard<'a, HttpServerRoutingTable> {
        self.routes.read().unwrap()
    }

    pub fn get_routing_table_mut<'a>(&'a self) -> RwLockWriteGuard<'a, HttpServerRoutingTable> {
        self.routes.write().unwrap()
    }
}

impl Deref for HttpServer {
    type Target = HttpServerImpl;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
