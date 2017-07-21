use std;
use std::error::Error;
use std::collections::HashMap;
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

use logging;

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
    let logger = logging::Logger::new("delegates::fire_handlers");

    let mut target_req = glue::Request::new();

    let uri = format!("{}", req.uri());
    let uri = uri.as_str();

    let remote_addr = format!("{}", req.remote_addr().unwrap());
    let remote_addr = remote_addr.as_str();

    let method = format!("{}", req.method());
    let method = method.as_str();

    logger.log(logging::Message::Info(format!("{} {} {}", remote_addr, method, uri)));

    target_req.set_context(Arc::into_raw(ctx.clone()));
    target_req.set_remote_addr(remote_addr);
    target_req.set_method(method);
    target_req.set_uri(uri);

    for hdr in req.headers().iter() {
        target_req.add_header(hdr.name(), hdr.value_string().as_str());
    }

    let mut session_id = String::new();

    match req.headers().get::<hyper::header::Cookie>() {
        Some(ref cookies) => {
            for (k, v) in cookies.iter() {
                target_req.add_cookie(k, v);
                if k == ctx.session_cookie_name.as_str() {
                    session_id = v.to_string();
                }
            }
        },
        None => {}
    }

    let url = uri.split("?").nth(0).unwrap();

    let raw_ep = ctx.router.lock().unwrap().get_raw_endpoint(url);
    let ep_id: i32;
    let mut read_body: bool;
    let init_session: bool;

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
            init_session = raw_ep.get_flag("init_session");
        },
        None => {
            ep_id = -1;
            read_body = false;
            init_session = false;

            let static_prefix = "/static"; // Hardcode it for now.

            if url.starts_with((static_prefix.to_string() + "/").as_str()) {
                if let Some(ref d) = ctx.static_dir {
                    return static_file::fetch(&ctx, &url[static_prefix.len()..], d.as_str());
                }
            }
        }
    }

    let mut cookies_to_append: HashMap<String, String> = HashMap::new();

    if init_session {
        let (sess, is_new) = match session_id.len() {
            0 => (ctx.session_storage.create_session(), true),
            _ => {
                match ctx.session_storage.get_session(session_id.as_str()) {
                    Some(s) => (s, false),
                    None => (ctx.session_storage.create_session(), true)
                }
            }
        };
        if is_new {
            cookies_to_append.insert(ctx.session_cookie_name.clone(), sess.read().unwrap().get_id());
        }
        target_req.set_session(Arc::into_raw(sess));
    }

    let max_request_body_size = ctx.max_request_body_size as usize;

    let (tx, rx) = oneshot::channel();
    let mut body: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let mut body_cloned = body.clone();
    let mut body_len = 0;

    //println!("read_body: {}", read_body);

    Box::new(req.body().for_each(move |chunk| {
        let mut body = body_cloned.lock().unwrap();

        if body_len + chunk.len() > max_request_body_size {
            body.clear();
            return Err(hyper::Error::TooLarge);
        }

        body_len += chunk.len();
        
        if read_body {
            body.extend_from_slice(&chunk);
        }

        Ok(())
    }).map_err(|e| e.description().to_string()).map(move |_| unsafe {
        let body = body.lock().unwrap();
        target_req.set_body(body.as_slice());

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
        let glue_resp = unsafe { glue::Response::from_raw(resp) };
        let mut headers = glue_resp.get_headers();

        headers.set_raw("X-Powered-By", "Ice Core");

        let cookies = glue_resp.get_cookies();
        let mut cookies_vec = Vec::new();

        for (k, v) in cookies.iter() {
            cookies_vec.push(k.clone() + "=" + v.as_str());
        }

        for (k, v) in cookies_to_append.iter() {
            cookies_vec.push(k.clone() + "=" + v.as_str());
        }

        headers.set(hyper::header::SetCookie(cookies_vec));

        let resp = Response::new().with_headers(headers).with_status(glue_resp.get_status());

        match glue_resp.get_file() {
            Some(p) => static_file::fetch_raw_unchecked(&ctx, resp, p.as_str()),
            None => {
                let resp_body = glue_resp.get_body();
                let mut headers = hyper::header::Headers::new();
                Box::new(futures::future::ok(resp.with_header(hyper::header::ContentLength(resp_body.len() as u64)).with_body(resp_body)))
            }
        }
    }).flatten())
}
