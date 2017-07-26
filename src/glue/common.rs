use std::ffi::CString;

pub struct HeaderIterator {
    pub headers: Vec<(CString, CString)>,
    pub pos: usize
}

#[no_mangle]
pub fn ice_glue_destroy_header_iterator(itr: *mut HeaderIterator) {
    unsafe { Box::from_raw(itr); }
}
