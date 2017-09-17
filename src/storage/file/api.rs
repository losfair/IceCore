use std::os::raw::c_char;
use std::ffi::CStr;
use hyper;
use logging;
use storage;

lazy_static! {
    static ref API_LOGGER: logging::Logger = logging::Logger::new("storage::file::api");
}

#[no_mangle]
pub unsafe extern "C" fn ice_storage_file_http_response_begin_send(
    req: &hyper::Request,
    resp: &mut hyper::Response,
    path: *const c_char
) -> bool {
    let path = CStr::from_ptr(path).to_str().unwrap();
    match storage::file::http_response::begin_send(req, resp, path) {
        Ok(_) => true,
        Err(e) => {
            API_LOGGER.log(logging::Message::Error(e));
            false
        }
    }
}
