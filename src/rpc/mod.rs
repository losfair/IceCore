use futures;
use futures::Future;
use futures::future::FutureResult;
use tarpc;

service! {
    rpc ping() -> bool;
}

#[derive(Clone)]
struct RpcServer {
}

impl FutureService for RpcServer {
    type PingFut = FutureResult<bool, tarpc::util::Never>;
    fn ping(&self) -> Self::PingFut {
        futures::future::ok(true)
    }
}
