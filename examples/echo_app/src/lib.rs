extern "C" {
    fn __ice_request_timeout(
        timeout: i64,
        cb: extern "C" fn (user_data: usize),
        user_data: usize
    );
    fn __ice_log(
        str_base: *const u8,
        str_len: usize
    );
    fn __ice_try_unwrap_callback_task(
        task_id: i32,
        target_ptr: *mut Option<extern "C" fn (user_data: usize)>,
        data_ptr: *mut usize
    ) -> i32;
}

fn write_log(s: &str) {
    let s = s.as_bytes();
    unsafe {
        __ice_log(
            &s[0],
            s.len()
        );
    }
}

#[no_mangle]
pub extern "C" fn app_task_dispatch(task_id: i32) -> i32 {
    let mut target: Option<extern "C" fn (user_data: usize)> = None;
    let mut data: usize = 0;

    let err = unsafe {
        __ice_try_unwrap_callback_task(
            task_id,
            &mut target,
            &mut data
        )
    };
    if err == 0 {
        (target.unwrap())(data);
    } else {
        write_log(&format!("Got task of unknown kind: {}", task_id));
    }
    0
}

extern "C" fn timeout_cb(user_data: usize) {
    write_log(&format!("Timeout cb {}", user_data));
    unsafe {
        __ice_request_timeout(1, timeout_cb, user_data + 1);
    }
}

#[no_mangle]
pub extern "C" fn app_init() -> i32 {
    unsafe {
        __ice_request_timeout(1, timeout_cb, 0);
    }
    0
}
