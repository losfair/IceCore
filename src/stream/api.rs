use super::{rstream, wstream};
use futures;

#[repr(C)]
pub struct RawTxRxPair {
    tx: *mut wstream::WriteStream,
    rx: *mut rstream::ReadStream
}

#[no_mangle]
pub fn ice_stream_create_pair(out: &mut RawTxRxPair) {
    let (tx, rx) = futures::sync::mpsc::channel(1024);

    out.tx = Box::into_raw(Box::new(tx.into()));
    out.rx = Box::into_raw(Box::new(rx.into()));
}
