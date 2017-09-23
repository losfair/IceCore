mod service;
mod param;
mod client;
pub mod call_context;
pub mod server_api;
pub mod call_context_api;
pub mod param_api;
pub mod client_api;

use std::ops::Deref;
use std::sync::Arc;
use std::collections::HashMap;
use futures;
use futures::Future;

#[derive(Clone)]
pub struct RpcServer {
    inner: Arc<RpcServerImpl>
}

pub struct RpcServerImpl {
    config: RpcServerConfig
}

#[derive(Default)]
pub struct RpcServerConfig {
    pub methods: HashMap<
        String,
        Box<Fn(Vec<param::Param>) -> Box<Future<Item = param::Param, Error = ()>> + Send + Sync>
    >
}

impl Deref for RpcServer {
    type Target = RpcServerImpl;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl RpcServer {
    pub fn new(config: RpcServerConfig) -> RpcServer {
        RpcServer {
            inner: Arc::new(RpcServerImpl {
                config: config
            })
        }
    }
    pub fn call(&self, name: &str, params: Vec<param::Param>) -> Box<Future<Item = param::Param, Error = ()>> {
        if let Some(target) = self.config.methods.get(name) {
            target(params)
        } else {
            Box::new(futures::future::ok(param::Param::Error(Box::new(param::Param::String(
                "Method not found".to_string()
            )))))
        }
    }
}

impl RpcServerConfig {
    pub fn new() -> RpcServerConfig {
        RpcServerConfig::default()
    }
}
