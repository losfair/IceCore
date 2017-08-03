use std::ffi::CString;

pub struct HeaderIterator {
    pub headers: Vec<(CString, CString)>,
    pub pos: usize
}

#[no_mangle]
pub unsafe fn ice_glue_destroy_header_iterator(itr: *mut HeaderIterator) {
    Box::from_raw(itr);
}
