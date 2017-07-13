use std::collections::HashMap;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};

type Pointer = usize;

pub struct Router {
    next_id: i32,
    routes: Pointer // PrefixTree
}

extern {
    fn ice_internal_create_prefix_tree() -> Pointer;
    fn ice_internal_destroy_prefix_tree(t: Pointer);
    fn ice_internal_prefix_tree_add_endpoint(t: Pointer, name: *const c_char, id: i32);
    fn ice_internal_prefix_tree_get_endpoint_id(t: Pointer, name: *const c_char) -> i32;
}

impl Router {
    pub fn new() -> Router {
        Router {
            next_id: 0,
            routes: unsafe { ice_internal_create_prefix_tree() }
        }
    }

    pub fn add_endpoint(&mut self, p: &str) -> i32 {
        unsafe {
            ice_internal_prefix_tree_add_endpoint(self.routes, CString::new(p).unwrap().as_ptr(), self.next_id);
        }

        self.next_id += 1;
        self.next_id - 1
    }

    pub fn get_endpoint_id(&self, p: &str) -> i32 {
        let id: i32;

        unsafe {
            id = ice_internal_prefix_tree_get_endpoint_id(self.routes, CString::new(p).unwrap().as_ptr());
        }

        id
    }
}

impl Drop for Router {
    fn drop(&mut self) {
        unsafe {
            ice_internal_destroy_prefix_tree(self.routes);
        }
        self.routes = 0;
    }
}
