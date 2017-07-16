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
use futures::Stream;

use hyper;
use hyper::server::{Request, Response};

use ice_server;
use glue;
use router;
use config;
use static_file;
use session_storage::{SessionStorage, Session};

pub type ServerHandle = *const Mutex<IceServer>;
pub type SessionHandle = *const RwLock<Session>;
pub type ContextHandle = *const ice_server::Context;
pub type Pointer = usize;

pub struct CallInfo {
    pub req: glue::Request,
    pub tx: oneshot::Sender<Pointer> // Response
}

pub fn fire_handlers(ctx: Arc<ice_server::Context>, req: Request) -> Box<Future<Item = Response, Error = String>> {
    let mut target_req = glue::Request::new();

    let uri = format!("{}", req.uri());
    let uri = uri.as_str();

    target_req.set_context(Arc::into_raw(ctx.clone()));
    target_req.set_remote_addr(format!("{}", req.remote_addr().unwrap()).as_str());
    target_req.set_method(format!("{}", req.method()).as_str());
    target_req.set_uri(uri);

    for hdr in req.headers().iter() {
        target_req.add_header(hdr.name(), hdr.value_string().as_str());
    }

    let url = uri.split("?").nth(0).unwrap();

    let raw_ep = ctx.router.read().unwrap().get_raw_endpoint(url);
    let ep_id: i32;
    let mut read_body: bool;

    match raw_ep {
        Some(raw_ep) => {
            let ep = raw_ep.to_endpoint();
            let mut pn_pos: usize = 0;

            for p in url.split("/").filter(|x| x.len() > 0) {
                if p.starts_with(":") {
                    target_req.add_param(ep.param_names[pn_pos].as_str(), &p[1..]);
                    pn_pos += 1;
                }
            }

            ep_id = ep.id;
            read_body = raw_ep.get_flag("read_body");
        },
        None => {
            ep_id = -1;
            read_body = false;

            let static_prefix = "/static"; // Hardcode it for now.

            if url.starts_with((static_prefix.to_string() + "/").as_str()) {
                if let Some(ref d) = ctx.static_dir {
                    return static_file::fetch(&ctx, &url[static_prefix.len()..], d.as_str());
                }
            }
        }
    }

    let (tx, rx) = oneshot::channel();
    let body: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let body_cloned = body.clone();

    //println!("read_body: {}", read_body);

    Box::new(req.body().for_each(move |chunk| {
        let mut body = body_cloned.lock().unwrap();
        if body.len() + chunk.len() > config::MAX_REQUEST_BODY_LEN {
            read_body = false;
            body.clear();
        }
        
        if read_body {
            body.extend_from_slice(&chunk);
        }

        Ok(())
    }).map_err(|e| e.description().to_string()).map(move |_| unsafe {
        target_req.set_body(body.lock().unwrap().as_slice());

        let call_info = Box::into_raw(Box::new(CallInfo {
            req: target_req,
            tx: tx
        }));

        glue::ice_glue_async_endpoint_handler(
            ep_id,
            call_info as Pointer
        );
        Ok(())
    }).join(rx.map_err(|e| e.description().to_string())).map(move |(_, resp): (Result<(), String>, Pointer)| {
        let resp = unsafe { glue::Response::from_raw(resp) };
        Response::new().with_headers(resp.get_headers()).with_status(resp.get_status()).with_body(resp.get_body())
    }))
    /*
    let after_read = Ok(()).map(move |_| unsafe {
        glue::ice_glue_async_endpoint_handler(
            ep_id,
            call_info as Pointer
        );
        Ok(())
    }).join(rx).map(|resp: Result<Pointer, _>| {
        unsafe { glue::Response::from_raw(resp.unwrap()); }
        Ok(())
        //Response::new().with_headers(resp.get_headers()).with_body(resp.get_body())
    }).map_err(|e| e.description().to_string());

    (match read_body {
        true => req.body().for_each(move |chunk| {
            body.lock().unwrap().extend_from_slice(chunk.to_vec().as_slice());
            Ok(())
        }).map(|_| after_read),
        false => after_read
    }).boxed()
    */
}
