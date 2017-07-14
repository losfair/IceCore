use std;
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};
use ice_server::IceServer;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use futures;
use futures::future::{FutureResult, Future};
use futures::{Async, Poll};
use futures::sync::oneshot;

use hyper;
use hyper::server::{Request, Response};

use ice_server;
use glue;

pub type ServerHandle = *const Mutex<IceServer>;
pub type Pointer = usize;

pub struct CallInfo {
    pub req: glue::Request,
    pub tx: oneshot::Sender<Pointer> // Response
}

pub fn fire_handlers(ctx: Arc<ice_server::Context>, req: Request) -> Box<Future<Item = glue::Response, Error = String>> {
    let mut target_req = glue::Request::new();

    target_req.set_remote_addr(format!("{}", req.remote_addr().unwrap()).as_str());
    target_req.set_method(format!("{}", req.method()).as_str());
    target_req.set_uri(format!("{}", req.uri()).as_str());

    for hdr in req.headers().iter() {
        target_req.add_header(hdr.name(), hdr.value_string().as_str());
    }

    let (tx, rx) = oneshot::channel();
    let call_info = Box::into_raw(Box::new(CallInfo {
        req: target_req,
        tx: tx
    }));

    unsafe {
        glue::ice_glue_async_endpoint_handler(
            ctx.router.read().unwrap().get_endpoint_id(format!("{}", req.uri()).as_str().split("?").nth(0).unwrap()),
            call_info as Pointer
        );
    }

    rx.map(|resp: Pointer| {
        unsafe { glue::Response::from_raw(resp) }
        //Response::new().with_headers(resp.get_headers()).with_body(resp.get_body())
    }).map_err(|e| e.description().to_string()).boxed()
}
