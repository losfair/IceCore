use futures;
use futures::Future;
use futures::future::FutureResult;
use tarpc;
use tarpc::util::FirstSocketAddr;
use rpc::param::Param;
use executor;

pub mod generated_service {
    use rpc::param::Param;

    service! {
        rpc ping() -> bool;
        rpc call(method: String, params: Vec<Param>) -> Param;
    }
}

#[derive(Clone)]
pub struct RpcService {
    server: super::RpcServer
}

impl self::generated_service::FutureService for RpcService {
    type PingFut = FutureResult<bool, tarpc::util::Never>;
    fn ping(&self) -> Self::PingFut {
        futures::future::ok(true)
    }

    type CallFut = Box<Future<Item = Param, Error = tarpc::util::Never>>;
    fn call(&self, method: String, params: Vec<Param>) -> Self::CallFut {
        Box::new(self.server.call(method.as_str(), params).then(|r| {
            match r {
                Ok(v) => futures::future::ok(v),
                Err(_) => panic!("Internal error: RPC call target should never return an `Err` value")
            }
        }))
    }
}

impl RpcService {
    pub fn new(server: &super::RpcServer) -> RpcService {
        RpcService {
            server: server.clone()
        }
    }

    pub fn start(self, addr: &str) {
        let addr = addr.first_socket_addr();
        executor::get_event_loop().spawn(move |reactor_handle| {
            use self::generated_service::FutureServiceExt;
            let (_, server) = self.listen(
                addr,
                &reactor_handle,
                tarpc::future::server::Options::default()
            ).unwrap();
            reactor_handle.spawn(server);
            futures::future::ok(())
        });
    }
}
