use std;
use std::os::raw::c_void;
use super::WriteStream;
use futures;
use futures::Stream;
use futures::future::Future;
use futures::Sink;
use executor;

#[no_mangle]
pub fn ice_stream_wstream_write(
    target: &WriteStream,
    data: *const u8,
    data_len: u32
) {
    let data = unsafe {
        std::slice::from_raw_parts(data, data_len as usize)
    };
    let f = target.lock().unwrap().clone().send(data.to_vec()).map(|_| ()).map_err(|_| ());

    executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe fn ice_stream_wstream_destroy(
    target: *mut WriteStream,
) {
    Box::from_raw(target);
}
