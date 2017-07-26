use std::error::Error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use ice_server::IceServer;
use std::ffi::CString;
use futures;
use futures::future::Future;
use futures::sync::oneshot;
use futures::Stream;
use std::rc::Rc;
use std::cell::RefCell;

use hyper;
use hyper::server::{Request, Response};

use logging;

use ice_server;
use glue_old;
use glue;
use static_file;
use time;
use session_storage::Session;

pub type ServerHandle = *const Mutex<IceServer>;
pub type SessionHandle = *const RwLock<Session>;
pub type ContextHandle = *const ice_server::Context;
pub type Pointer = usize;

pub struct CallInfo {
    pub req: Box<glue::request::Request>,
    pub tx: oneshot::Sender<Pointer> // Response
}

pub fn fire_handlers(ctx: Arc<ice_server::Context>, req: Request) -> Box<Future<Item = Response, Error = String>> {
    let logger = logging::Logger::new("delegates::fire_handlers");

    let uri = format!("{}", req.uri());

    let remote_addr = format!("{}", req.remote_addr().unwrap());

    let method = format!("{}", req.method());

    if ctx.log_requests {
        logger.log(logging::Message::Info(format!("{} {} {}", remote_addr.as_str(), method.as_str(), uri.as_str())));
    }

    /*
    target_req.set_context(Arc::into_raw(ctx.clone()));
    target_req.set_remote_addr(remote_addr);
    target_req.set_method(method);
    target_req.set_uri(uri);
    */

    let req_headers = req.headers().clone();

    let mut session_id = String::new();
    let mut cookies = HashMap::new();

    match req.headers().get::<hyper::header::Cookie>() {
        Some(ref _cookies) => {
            for (k, v) in _cookies.iter() {
                cookies.insert(k.to_string(), CString::new(v).unwrap());
                if k == ctx.session_cookie_name.as_str() {
                    session_id = v.to_string();
                }
            }
        },
        None => {}
    }

    let url = uri.split("?").nth(0).unwrap().to_string();
    let url = url.as_str();

    let ep_id: i32;
    let read_body: bool;
    let init_session: bool;
    let ep_path;

    match ctx.router.lock().unwrap().borrow_endpoint(url) {
        Some(ref ep) => {
            ep_id = ep.id;
            read_body = *ep.flags.get("read_body").unwrap_or(&false);
            init_session = *ep.flags.get("init_session").unwrap_or(&false);
            ep_path = ep.name.clone();
        },
        None => {
            ep_id = -1;
            read_body = false;
            init_session = false;
            ep_path = "[Unknown]".to_string();

            let static_prefix = "/static"; // Hardcode it for now.

            if url.starts_with((static_prefix.to_string() + "/").as_str()) {
                if let Some(ref d) = ctx.static_dir {
                    return static_file::fetch(&ctx, &url[static_prefix.len()..], d.as_str());
                }
            }
        }
    }

    ctx.stats.inc_endpoint_hit(ep_path.as_str());

    let mut cookies_to_append: HashMap<String, String> = HashMap::new();

    let sess = if init_session {
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
        Some(sess)
    } else {
        None
    };

    let max_request_body_size = ctx.max_request_body_size as usize;

    let (tx, rx) = oneshot::channel();
    let body: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    let body_cloned = body.clone();
    let mut body_len = 0;

    //println!("read_body: {}", read_body);
    let ctx_cloned = ctx.clone();
    let req_headers_cloned = req_headers.clone();
    
    let start_micros = time::micros();

    let reader: Box<Future<Item = (), Error = hyper::Error>> = match read_body {
        true => Box::new(req.body().for_each(move |chunk| {
                let mut body = body_cloned.borrow_mut();

                if body_len + chunk.len() > max_request_body_size {
                    body.clear();
                    return Err(hyper::Error::TooLarge);
                }

                body_len += chunk.len();
                body.extend_from_slice(&chunk);

                Ok(())
            }).map(|_| ())),
        false => Box::new(futures::future::ok(()))
    };

    Box::new(reader.map_err(|e| e.description().to_string()).map(move |_| unsafe {
        let body = body.borrow();

        let call_info = Box::into_raw(Box::new(CallInfo {
            req: glue::request::Request {
                uri: CString::new(uri).unwrap(),
                remote_addr: CString::new(remote_addr).unwrap(),
                method: CString::new(method).unwrap(),
                headers: req_headers_cloned,
                cookies: cookies,
                body: body.clone(),
                context: ctx_cloned,
                session: sess,
                cache: glue::request::RequestCache::default()
            }.into_boxed(),
            tx: tx
        }));

        glue_old::ice_glue_async_endpoint_handler(
            ep_id,
            call_info as Pointer
        );
        Ok(())
    }).join(rx.map_err(|e| e.description().to_string())).map(move |(_, resp): (Result<(), String>, Pointer)| {
        let glue_resp = unsafe { glue_old::Response::from_raw(resp) };
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

        let end_micros = time::micros();
        ctx.stats.add_endpoint_processing_time(ep_path.as_str(), end_micros - start_micros);

        let resp = Response::new().with_headers(headers).with_status(glue_resp.get_status());

        match glue_resp.get_file() {
            Some(p) => {
                let etag = match req_headers.get::<hyper::header::IfNoneMatch>() {
                    Some(v) => {
                        match v {
                            &hyper::header::IfNoneMatch::Any => None,
                            &hyper::header::IfNoneMatch::Items(ref v) => {
                                if v.len() == 0 {
                                    None
                                } else {
                                    Some(v[0].tag().to_string())
                                }
                            }
                        }
                    },
                    None => None
                };
                static_file::fetch_raw_unchecked(&ctx, resp, p.as_str(), etag)
            },
            None => {
                let resp_body = glue_resp.get_body();
                Box::new(futures::future::ok(resp.with_header(hyper::header::ContentLength(resp_body.len() as u64)).with_body(resp_body)))
            }
        }
    }).flatten())
}
