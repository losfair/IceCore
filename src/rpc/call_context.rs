use futures;
use rpc::param::Param;

pub struct CallContext {
    pub params: Vec<Param>,
    result_tx: futures::sync::oneshot::Sender<Param>
}

impl CallContext {
    pub fn new(params: Vec<Param>) -> (CallContext, futures::sync::oneshot::Receiver<Param>) {
        let (tx, rx) = futures::sync::oneshot::channel();
        (CallContext {
            params: params,
            result_tx: tx
        }, rx)
    }

    pub fn end(self, ret: Param) {
        match self.result_tx.send(ret) {
            Ok(_) => {},
            Err(_) => {}
        };
    }
}
