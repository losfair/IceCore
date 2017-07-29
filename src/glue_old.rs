type Pointer = usize;

#[no_mangle]
extern {
    pub fn ice_glue_async_endpoint_handler(id: i32, call_info: Pointer);
}
