use std;
use std::collections::HashMap;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use std::any::Any;
use glue::serialize;
use ice_server;

#[derive(Default)]
pub struct InteropContext {
    name: String,
    tx_data: HashMap<String, String>,
    rx_data: HashMap<String, String>,
    tx_cache: Option<Vec<u8>>,
    rx_cache: Option<Vec<u8>>,
    tx_field_cache: HashMap<String, CString>,
    rx_field_cache: HashMap<String, CString>
}

impl Into<Box<Any>> for Box<InteropContext> {
    fn into(self) -> Box<Any> {
        self
    }
}

impl InteropContext {
    pub fn with_name(name: String) -> InteropContext {
        let mut ctx = InteropContext::default();
        ctx.name = name;
        ctx
    }

    pub fn serialize_tx(&mut self) -> *const u8 {
        let data = serialize::std_map(self.tx_data.iter(), self.tx_data.len());
        self.tx_cache = Some(data);
        self.tx_cache.as_ref().unwrap().as_ptr()
    }

    pub fn serialize_rx(&mut self) -> *const u8 {
        let data = serialize::std_map(self.rx_data.iter(), self.rx_data.len());
        self.rx_cache = Some(data);
        self.rx_cache.as_ref().unwrap().as_ptr()
    }

    pub fn set_tx_field(&mut self, k: String, v: String) {
        self.tx_data.insert(k, v);
    }

    pub fn set_rx_field(&mut self, k: String, v: String) {
        self.rx_data.insert(k, v);
    }

    pub fn get_tx_field(&mut self, k: &str) -> *const c_char {
        let s = match self.tx_data.get(k) {
            Some(ref v) => CString::new(v.as_str()).unwrap(),
            None => return std::ptr::null()
        };
        let addr = s.as_ptr();

        self.tx_field_cache.insert(k.to_string(), s);
        addr
    }

    pub fn get_rx_field(&mut self, k: &str) -> *const c_char {
        let s = match self.rx_data.get(k) {
            Some(ref v) => CString::new(v.as_str()).unwrap(),
            None => return std::ptr::null()
        };
        let addr = s.as_ptr();

        self.rx_field_cache.insert(k.to_string(), s);
        addr
    }

    pub fn run_hooks(target: Box<InteropContext>, app_ctx: &ice_server::Context) -> Box<InteropContext> {
        let hook_name = "interop_".to_string() + target.name.as_str();

        app_ctx.modules.run_hooks_by_name(
            hook_name.as_str(),
            target
        )
    }
}

#[no_mangle]
pub unsafe fn ice_glue_interop_create_context_with_name(name: *const c_char) -> *mut InteropContext {
    Box::into_raw(Box::new(InteropContext::with_name(
        CStr::from_ptr(name).to_str().unwrap().to_string()
    )))
}

#[no_mangle]
pub unsafe fn ice_glue_interop_destroy_context(ctx: *mut InteropContext) {
    Box::from_raw(ctx);
}

#[no_mangle]
pub unsafe fn ice_glue_interop_run_hooks(ctx: *mut InteropContext, app_ctx: *const ice_server::Context) {
    let ctx = Box::from_raw(ctx);
    let app_ctx = &*app_ctx;

    Box::into_raw(
        InteropContext::run_hooks(ctx, app_ctx)
    );
}

#[no_mangle]
pub unsafe fn ice_glue_interop_set_tx_field(ctx: *mut InteropContext, k: *const c_char, v: *const c_char) {
    let ctx = &mut *ctx;
    let k = CStr::from_ptr(k).to_str().unwrap().to_string();
    let v = CStr::from_ptr(v).to_str().unwrap().to_string();

    ctx.set_tx_field(k, v);
}

#[no_mangle]
pub unsafe fn ice_glue_interop_set_rx_field(ctx: *mut InteropContext, k: *const c_char, v: *const c_char) {
    let ctx = &mut *ctx;
    let k = CStr::from_ptr(k).to_str().unwrap().to_string();
    let v = CStr::from_ptr(v).to_str().unwrap().to_string();

    ctx.set_rx_field(k, v);
}

#[no_mangle]
pub unsafe fn ice_glue_interop_get_tx_field(ctx: *mut InteropContext, k: *const c_char) -> *const c_char {
    let ctx = &mut *ctx;
    let k = CStr::from_ptr(k).to_str().unwrap();

    ctx.get_tx_field(k)
}

#[no_mangle]
pub unsafe fn ice_glue_interop_get_rx_field(ctx: *mut InteropContext, k: *const c_char) -> *const c_char {
    let ctx = &mut *ctx;
    let k = CStr::from_ptr(k).to_str().unwrap();

    ctx.get_rx_field(k)
}

#[no_mangle]
pub unsafe fn ice_glue_interop_read_tx(ctx: *mut InteropContext) -> *const u8 {
    let ctx = &mut *ctx;
    ctx.serialize_tx()
}

#[no_mangle]
pub unsafe fn ice_glue_interop_read_rx(ctx: *mut InteropContext) -> *const u8 {
    let ctx = &mut *ctx;
    ctx.serialize_rx()
}
