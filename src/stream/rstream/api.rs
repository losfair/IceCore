use std::os::raw::c_void;
use super::ReadStream;
use futures;
use futures::Stream;
use futures::future::Future;
use executor;

#[no_mangle]
pub extern "C" fn ice_stream_rstream_begin_recv(
    target: &mut ReadStream,
    cb_on_data: extern fn (call_with: usize, data: *const u8, data_len: u32) -> bool,
    cb_on_end: extern fn (call_with: usize),
    cb_on_error: Option<extern fn (call_with: usize)>,
    call_with: *const c_void
) {
    let call_with = call_with as usize;
    let receiver = target.take_receiver();

    let f = receiver.for_each(move |data| {
        let result = cb_on_data(call_with, data.as_ptr(), data.len() as u32);
        if !result {
            futures::future::err(())
        } else {
            futures::future::ok(())
        }
    })
    .map(move |_| cb_on_end(call_with))
    .or_else(move |_| {
        if cb_on_error.is_some() {
            cb_on_error.unwrap()(call_with);
        }
        Ok(())
    });

    executor::get_event_loop().spawn(move |_| f);
}

#[no_mangle]
pub unsafe extern "C" fn ice_stream_rstream_destroy(
    target: *mut ReadStream
) {
    Box::from_raw(target);
}
