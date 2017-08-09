use std::error::Error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ice_server::IceServer;
use std::ffi::CString;
use futures;
use futures::future::Future;
use futures::sync::oneshot;
use futures::Stream;
use std;
use std::rc::Rc;
use std::cell::RefCell;
use std::os::raw::{c_void, c_char};
use std::sync::atomic;
use tokio_core::reactor;

use hyper;
use hyper::server::{Request, Response};

use logging;

use ice_server;
use glue;
use static_file;
use time;
use session_storage::Session;

pub type ServerHandle = *const Mutex<IceServer>;
pub type SessionHandle = *const Mutex<Session>;
pub type ContextHandle = *const ice_server::Context;

/*
unsafe fn check_and_free_cstring(s: &mut *mut c_char) {
    if !s.is_null() {
        CString::from_raw(*s);
        *s = std::ptr::null_mut();
    }
}
*/

#[repr(C)]
pub struct BasicRequestInfo {
    uri: *const c_char,
    remote_addr: *const c_char,
    method: *const c_char,
    response: *mut glue::response::Response,
    custom_properties: *const glue::common::CustomProperties
}

impl BasicRequestInfo {
    fn new(custom_properties: &glue::common::CustomProperties) -> BasicRequestInfo {
        BasicRequestInfo {
            uri: std::ptr::null(),
            remote_addr: std::ptr::null(),
            method: std::ptr::null(),
            response: std::ptr::null_mut(),
            custom_properties: custom_properties
        }
    }

    fn set_uri(&mut self, uri: &CString) {
        self.uri = uri.as_ptr();
    }

    fn set_remote_addr(&mut self, remote_addr: &CString) {
        self.remote_addr = remote_addr.as_ptr();
    }

    fn set_method(&mut self, method: &CString) {
        self.method = method.as_ptr();
    }

    unsafe fn move_out_response(&mut self) -> Option<Box<glue::response::Response>> {
        if self.response.is_null() {
            None
        } else {
            let ret = Box::from_raw(self.response);
            self.response = std::ptr::null_mut();
            Some(ret)
        }
    }

    pub fn has_response(&self) -> bool {
        !self.response.is_null()
    }
}

impl Drop for BasicRequestInfo {
    fn drop(&mut self) {
        unsafe {
            if !self.response.is_null() {
                Box::from_raw(self.response);
            }
        }
    }
}

pub struct CallInfo {
    pub req: Box<glue::request::Request>,
    pub custom_app_data: CustomAppData,
    pub tx: oneshot::Sender<Box<glue::response::Response>> // Response
}

#[derive(Clone)]
pub struct CustomAppData {
    handle: Arc<atomic::AtomicUsize>
}

impl CustomAppData {
    pub fn empty() -> CustomAppData {
        CustomAppData {
            handle: Arc::new(atomic::AtomicUsize::new(0))
        }
    }

    pub fn get_raw(&self) -> *const c_void {
        self.handle.load(atomic::Ordering::SeqCst) as *const c_void
    }

    pub fn set_raw(&self, ptr: *const c_void) {
        self.handle.store(ptr as usize, atomic::Ordering::SeqCst);
    }
}

pub fn fire_handlers(ctx: Arc<ice_server::Context>, local_ctx: Rc<ice_server::LocalContext>, req: Request) -> Box<Future<Item = Response, Error = String>> {
    let logger = logging::Logger::new("delegates::fire_handlers");
    let custom_properties = Arc::new(glue::common::CustomProperties::default());

    let uri = format!("{}", req.uri());
    let remote_addr = format!("{}", req.remote_addr().unwrap());
    let method = format!("{}", req.method());

    let uri_c = CString::new(uri.as_str()).unwrap();
    let remote_addr_c = CString::new(remote_addr.as_str()).unwrap();
    let method_c = CString::new(method.as_str()).unwrap();

    if ctx.log_requests {
        logger.log(logging::Message::Info(format!("{} {} {}", remote_addr.as_str(), method.as_str(), uri.as_str())));
    }

    {
        let mut basic_info = BasicRequestInfo::new(&custom_properties);

        basic_info.set_uri(&uri_c);
        basic_info.set_remote_addr(&remote_addr_c);
        basic_info.set_method(&method_c);

        ctx.cervus_modules.read().unwrap().run_hook(ice_server::Hook::BeforeRequest(&mut basic_info));

        unsafe {
            match basic_info.move_out_response() {
                Some(resp) => {
                    return resp.into_hyper_response(&ctx, &local_ctx, None);
                },
                None => {}
            }
        }
    }

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

    match ctx.router.borrow_endpoint(url) {
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
            cookies_to_append.insert(ctx.session_cookie_name.clone(), sess.lock().unwrap().get_id());
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

    let custom_app_data = ctx.custom_app_data.clone();

    //println!("read_body: {}", read_body);
    let ctx_cloned = ctx.clone();
    let req_headers_cloned = req_headers.clone();
    let async_endpoint_cb = local_ctx.async_endpoint_cb.clone();

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

    let endpoint_timeout: Box<Future<Item = Box<glue::response::Response>, Error = String>> = match ctx.endpoint_timeout_ms {
        0 => Box::new(futures::future::empty()),
        _ => Box::new(
            reactor::Timeout::new(std::time::Duration::from_millis(ctx.endpoint_timeout_ms), &local_ctx.ev_loop_handle).unwrap().map(|_| {
                let mut resp = Box::new(glue::response::Response::new());
                resp.status = 500;
                resp.body = "Timeout".as_bytes().to_vec();
                resp
            }).map_err(|e| e.description().to_string())
        )
    };

    let cp_cloned = custom_properties.clone();

    Box::new(reader.map_err(|e| e.description().to_string()).and_then(move |_| {
        let call_info = Box::into_raw(Box::new(CallInfo {
            req: glue::request::Request {
                uri: uri_c,
                remote_addr: remote_addr_c,
                method: method_c,
                headers: req_headers_cloned,
                cookies: cookies,
                custom_properties: cp_cloned,
                body: Box::new(body),
                context: ctx_cloned,
                session: sess,
                cache: glue::request::RequestCache::default()
            }.into_boxed(),
            custom_app_data: custom_app_data,
            tx: tx
        }));

        async_endpoint_cb(ep_id, call_info);
        rx.map_err(|e| e.description().to_string())
            .select(endpoint_timeout)
            .map(|r| r.0)
            .map_err(|e| e.0)
    }).map(move |mut glue_resp: Box<glue::response::Response>| {
        for (k, v) in cookies_to_append.iter() {
            glue_resp.cookies.insert(k.clone(), v.clone());
        }

        let cp = custom_properties;
        ctx.cervus_modules.read().unwrap().run_hook(ice_server::Hook::AfterResponse(&mut glue_resp, &cp));

        glue_resp.into_hyper_response(&ctx, &local_ctx, Some(req_headers))
    }).flatten())
}
