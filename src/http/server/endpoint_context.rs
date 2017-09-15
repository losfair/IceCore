use std;
use std::error::Error;
use futures;
use futures::Future;
use hyper;
use metadata;

pub struct EndpointContext {
    response_sender: futures::sync::oneshot::Sender<Result<hyper::Response, Box<Error + Send>>>,
    request: Box<hyper::Request>
}

impl EndpointContext {
    pub fn new_pair(req: Box<hyper::Request>) -> (EndpointContext, Box<Future<Item = hyper::Response, Error = hyper::Error>>) {
        let (resp_tx, resp_rx) = futures::sync::oneshot::channel();
        let version_string = "Ice/".to_string() + metadata::VERSION;

        (EndpointContext {
            response_sender: resp_tx,
            request: req
        }, Box::new(
            resp_rx.map_err(|e| hyper::Error::from(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.description()
                )
            )).and_then(move |r| {
                match r {
                    Ok(mut v) => Ok({
                        v.headers_mut().set_raw("Server", version_string.as_str());
                        v
                    }),
                    Err(e) => Ok(
                        hyper::Response::new().with_status(
                            hyper::StatusCode::InternalServerError
                        ).with_body(e.description().to_string())
                    )
                }
            })
        ))
    }

    pub fn get_request(&self) -> &hyper::Request {
        &*self.request
    }

    pub fn end(self, resp: hyper::Response) -> bool {
        match self.response_sender.send(Ok(resp)) {
            Ok(_) => true,
            Err(_) => false
        }
    }

    fn _require_send(self) {
        let _: Box<Send> = Box::new(self);
    }
}
