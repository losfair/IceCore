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
    fn ice_internal_prefix_tree_add_endpoint(t: Pointer, name: *const c_char, id: i32) -> Pointer;
    fn ice_internal_prefix_tree_get_endpoint_id(t: Pointer, name: *const c_char) -> i32;
    fn ice_internal_prefix_tree_get_endpoint(t: Pointer, name: *const c_char) -> Pointer;
    pub fn ice_internal_prefix_tree_endpoint_get_id(ep: Pointer) -> i32;
    pub fn ice_internal_prefix_tree_endpoint_set_flag(ep: Pointer, name: *const c_char, value: bool);
    pub fn ice_internal_prefix_tree_endpoint_get_flag(ep: Pointer, name: *const c_char) -> bool;
    fn ice_internal_prefix_tree_endpoint_create_param_name_iterator(ep: Pointer) -> Pointer;
    fn ice_internal_prefix_tree_endpoint_destroy_param_name_iterator(itr: Pointer);
    fn ice_internal_prefix_tree_endpoint_param_name_iterator_next(ep: Pointer, itr: Pointer) -> *const c_char;
}

impl Router {
    pub fn new() -> Router {
        Router {
            next_id: 0,
            routes: unsafe { ice_internal_create_prefix_tree() }
        }
    }

    pub fn add_endpoint(&mut self, p: &str) -> Pointer {
        let ep: Pointer;

        unsafe {
            ep = ice_internal_prefix_tree_add_endpoint(self.routes, CString::new(p).unwrap().as_ptr(), self.next_id);
        }

        self.next_id += 1;
        
        ep
    }

    pub fn get_endpoint_id(&self, p: &str) -> i32 {
        let id: i32;

        unsafe {
            id = ice_internal_prefix_tree_get_endpoint_id(self.routes, CString::new(p).unwrap().as_ptr());
        }

        id
    }

    pub fn get_raw_endpoint(&self, p: &str) -> Option<RawEndpoint> {
        let raw_ep = unsafe { ice_internal_prefix_tree_get_endpoint(self.routes, CString::new(p).unwrap().as_ptr()) };
        if raw_ep == 0 {
            None
        } else {
            Some(RawEndpoint {
                handle: raw_ep
            })
        }
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

pub struct Endpoint {
    pub id: i32,
    pub param_names: Vec<String>
}

// TODO: Lifetime
pub struct RawEndpoint {
    pub handle: Pointer
}

impl RawEndpoint {
    pub fn to_endpoint(&self) -> Endpoint {
        Endpoint {
            id: unsafe { ice_internal_prefix_tree_endpoint_get_id(self.handle) },
            param_names: Vec::new()
        }

        // Segfaults...
        /*
        let mut param_names: Vec<String> = Vec::new();

        unsafe {
            let itr = ice_internal_prefix_tree_endpoint_create_param_name_iterator(self.handle);
            loop {
                let pn = ice_internal_prefix_tree_endpoint_param_name_iterator_next(self.handle, itr);
                if pn.is_null() {
                    break;
                }
                param_names.push(CStr::from_ptr(pn).to_str().unwrap().to_string());
            }
            ice_internal_prefix_tree_endpoint_destroy_param_name_iterator(self.handle);

            Endpoint {
                id: ice_internal_prefix_tree_endpoint_get_id(self.handle),
                param_names: param_names
            }
        }*/
    }

    pub fn get_flag(&self, k: &str) -> bool {
        unsafe { ice_internal_prefix_tree_endpoint_get_flag(self.handle, CString::new(k).unwrap().as_ptr()) }
    }

    pub fn set_flag(&self, k: &str, v: bool) {
        unsafe { ice_internal_prefix_tree_endpoint_set_flag(self.handle, CString::new(k).unwrap().as_ptr(), v); }
    }
}
