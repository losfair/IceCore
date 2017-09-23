use std::net::SocketAddr;
use std::error::Error;
use futures;
use futures::Future;
use tarpc;
use tarpc::util::FirstSocketAddr;
use tarpc::future::client::ClientExt;
use executor;
use rpc::service::generated_service;
use rpc::param::Param;

#[derive(Clone)]
pub struct RpcClient {
    addr: SocketAddr
}

pub struct RpcClientConnection {
    client: Option<self::generated_service::FutureClient>
}

impl RpcClient {
    pub fn new(addr: &str) -> RpcClient {
        RpcClient {
            addr: addr.first_socket_addr()
        }
    }

    pub fn connect(&self) -> Box<Future<Item = RpcClientConnection, Error = String>> {
        let (tx, rx) = futures::sync::oneshot::channel();
        let addr = self.addr.clone();

        executor::get_event_loop().spawn(move |handle| {
            let options = tarpc::future::client::Options::default().handle(handle.clone());

            self::generated_service::FutureClient::connect(
                addr,
                options
            )
            .then(move |result| {
                tx.send(match result {
                    Ok(client) => Ok(client),
                    Err(e) => Err(e.description().to_string())
                }).unwrap();
                futures::future::ok(())
            })
        });

        Box::new(
            rx.map_err(|e| e.description().to_string())
            .and_then(|v| v)
            .map(|v| RpcClientConnection {
                client: Some(v)
            })
        )
    }
}

impl RpcClientConnection {
    pub fn call(&self, method: String, params: Vec<Param>) -> Box<Future<Item = Param, Error = String> + Send> {
        match self.client {
            Some(ref v) => Box::new(
                v.call(method, params)
                .map_err(|e| {
                    e.description().to_string()
                })
            ),
            None => Box::new(futures::future::err("Invalid client".to_string()))
        }
    }
}
