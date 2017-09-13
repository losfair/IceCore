mod executor;
mod config;
mod path_utils;
mod router;
pub mod api;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::ops::Deref;
use futures::Future;
use http::server::executor::HttpServerExecutor;
use http::server::config::HttpServerConfig;
use hyper;
use prefix_tree::PrefixTree;

pub type RouteCallbackFn = Fn(hyper::Request) -> Box<Future<Item = hyper::Response, Error = hyper::Error>>;

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
    executors: Vec<HttpServerExecutor>
}

pub struct HttpServerRoutingTable {
    routes: PrefixTree<String, RouteInfo>
}

pub struct RouteInfo {
    method: hyper::Method,
    path: Vec<String>,
    callback: Box<RouteCallbackFn>
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
            executors: executors
        })
    }
}

impl Deref for HttpServer {
    type Target = HttpServerImpl;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl HttpServerRoutingTable {
    pub fn new() -> HttpServerRoutingTable {
        HttpServerRoutingTable {
            routes: PrefixTree::new()
        }
    }
}
