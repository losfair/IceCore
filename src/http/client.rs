use std;
use std::rc::Rc;
use std::sync::Mutex;
use hyper;
use futures;
use futures::Future;
use futures::Stream;
use futures::Sink;
use tokio_core::reactor;
use executor;
use http::request::Request;

pub struct Client {
    op_tx: Mutex<futures::sync::mpsc::Sender<Op>>
}

struct Op {
    op_type: OpType,
    result_tx: futures::sync::oneshot::Sender<ResultType>
}

enum OpType {
    Request(hyper::Request)
}

enum ResultType {
    Response(hyper::Response),
    Error(hyper::Error)
}

impl Client {
    pub fn new() -> Client {
        let (op_tx, op_rx) = futures::sync::mpsc::channel(1024);

        executor::get_event_loop().spawn(move |handle| {
            Client::worker(handle, op_rx)
        });

        Client {
            op_tx: Mutex::new(op_tx)
        }
    }

    pub fn request(&self, req: hyper::Request) -> Box<Future<Item = hyper::Response, Error = hyper::Error>> {
        let (result_tx, result_rx) = futures::sync::oneshot::channel();

        let op_tx = self.op_tx.lock().unwrap().clone();
        executor::get_event_loop().spawn(move |_| {
            op_tx.send(Op {
                op_type: OpType::Request(req),
                result_tx: result_tx
            }).map(|_| ()).map_err(|_| ())
        });

        Box::new(result_rx.then(|v| {
            match v {
                Ok(v) => {
                    if let ResultType::Response(v) = v {
                        Ok(v)
                    } else if let ResultType::Error(e) = v {
                        Err(e)
                    } else {
                        panic!()
                    }
                },
                Err(e) => panic!()
            }
        }))
    }

    fn worker(handle: &reactor::Handle, op_rx: futures::sync::mpsc::Receiver<Op>) -> Box<Future<Item = (), Error = ()>> {
        let client = Rc::new(hyper::Client::new(handle));

        Box::new(
            op_rx.for_each(move |op| {
                let result_tx = op.result_tx;

                match op.op_type {
                    OpType::Request(req) => {
                        client.request(req).map(move |resp| {
                            result_tx.send(ResultType::Response(resp))
                        }).map(|_| ()).map_err(|_| ())
                    }
                }
            }).map(|_| ()).map_err(|_| ())
        )
    }
}

#[no_mangle]
pub extern "C" fn ice_http_client_create() -> *mut Client {
    Box::into_raw(Box::new(Client::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ice_http_client_destroy(client: *mut Client) {
    Box::from_raw(client);
}

#[no_mangle]
pub extern "C" fn ice_http_request_create() -> *mut Request {
    Box::into_raw(Box::new(Request::new()))
}

#[no_mangle]
pub extern "C" fn ice_http_client_request(
    client: &Client,
    request: *mut Request
) {
    let request = unsafe {
        Box::from_raw(request)
    };
}
