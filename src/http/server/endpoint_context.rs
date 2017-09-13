use std;
use std::error::Error;
use futures;
use futures::Future;
use hyper;

pub struct EndpointContext {
    response_sender: futures::sync::oneshot::Sender<Result<hyper::Response, Box<Error>>>,
    request: Box<hyper::Request>
}

impl EndpointContext {
    pub fn new_pair(req: Box<hyper::Request>) -> (EndpointContext, Box<Future<Item = hyper::Response, Error = hyper::Error>>) {
        let (resp_tx, resp_rx) = futures::sync::oneshot::channel();

        (EndpointContext {
            response_sender: resp_tx,
            request: req
        }, Box::new(
            resp_rx.map_err(|e| hyper::Error::from(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.description()
                )
            )).and_then(|r| {
                match r {
                    Ok(v) => Ok(v),
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

    pub fn end(self, resp: hyper::Response) {
        self.response_sender.send(Ok(resp)).unwrap();
    }
}
