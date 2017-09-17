use std;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use serde_json;
use trait_handle::TraitHandle;
use view::html_template::tera::TeraEngine;
use super::{HtmlTemplateEngine, HtmlTemplateError};
use logging;

lazy_static! {
    static ref API_LOGGER: logging::Logger = logging::Logger::new("view::html_template::api");
}

#[no_mangle]
pub unsafe extern "C" fn ice_view_html_template_create_engine(
    engine_name: *const c_char
) -> *mut TraitHandle<HtmlTemplateEngine> {
    let engine_name = CStr::from_ptr(engine_name).to_str().unwrap();

    let engine: Box<HtmlTemplateEngine> = match engine_name {
        "tera" => Box::new(TeraEngine::new()),
        _ => return std::ptr::null_mut()
    };

    Box::into_raw(Box::new(engine.into()))
}

#[no_mangle]
pub unsafe extern "C" fn ice_view_html_template_destroy_engine(
    engine: *mut TraitHandle<HtmlTemplateEngine>
) {
    Box::from_raw(engine);
}

#[no_mangle]
pub unsafe extern "C" fn ice_view_html_template_add(
    engine: &mut TraitHandle<HtmlTemplateEngine>,
    name: *const c_char,
    content: *const c_char
) -> bool {
    let name = CStr::from_ptr(name).to_str().unwrap();
    let content = CStr::from_ptr(content).to_str().unwrap();

    match engine.add(name, content) {
        Ok(_) => true,
        Err(e) => {
            API_LOGGER.log(logging::Message::Error(e.into()));
            false
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_view_html_template_add_file(
    engine: &mut TraitHandle<HtmlTemplateEngine>,
    path: *const c_char
) -> bool {
    let path = CStr::from_ptr(path).to_str().unwrap();

    match engine.add_file(path) {
        Ok(_) => true,
        Err(e) => {
            API_LOGGER.log(logging::Message::Error(e.into()));
            false
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ice_view_html_template_render_json_to_owned(
    engine: &mut TraitHandle<HtmlTemplateEngine>,
    name: *const c_char,
    data: *const c_char
) -> *mut c_char {
    let name = CStr::from_ptr(name).to_str().unwrap();
    let data = CStr::from_ptr(data).to_str().unwrap();

    let data: serde_json::Value = match serde_json::from_str(data) {
        Ok(d) => d,
        Err(e) => {
            API_LOGGER.log(logging::Message::Error(e.description().to_string()));
            return std::ptr::null_mut();
        }
    };

    match engine.render(name, &data) {
        Ok(v) => CString::new(v).unwrap().into_raw(),
        Err(e) => {
            API_LOGGER.log(logging::Message::Error(e.into()));
            std::ptr::null_mut()
        }
    }
}
