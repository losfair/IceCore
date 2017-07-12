use std::sync::{Arc, Mutex};
use ice_server::IceServer;
use std::os::raw::c_char;

pub type ServerHandle = Arc<Mutex<IceServer>>;
pub type Pointer = usize;

#[no_mangle]
extern {
    fn ice_glue_create_request() -> Pointer;
    fn ice_glue_destroy_request(req: Pointer);
    fn ice_glue_request_set_remote_addr(req: Pointer, addr: *const c_char);
    fn ice_glue_request_set_method(req: Pointer, m: *const c_char);
    fn ice_glue_request_set_uri(req: Pointer, uri: *const c_char);
    fn ice_glue_request_add_header(req: Pointer, k: *const c_char, v: *const c_char);
    fn ice_glue_request_to_raw(req: Pointer) -> Pointer;
    fn ice_glue_request_destroy_raw(raw: Pointer);
}



/*
#[no_mangle]
pub extern fn ice_delegate_endpoint_handler(server: ServerHandle);
*/