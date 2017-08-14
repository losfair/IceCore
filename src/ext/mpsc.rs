use std;

pub struct Mpsc {
    tx: std::sync::mpsc::Sender<*mut UnknownData>,
    rx: std::sync::mpsc::Receiver<*mut UnknownData>
}

#[derive(Clone)]
pub struct Sender {
    tx: std::sync::mpsc::Sender<*mut UnknownData>
}

pub struct UnknownData {}

impl Mpsc {
    pub fn new() -> Mpsc {
        let (tx, rx) = std::sync::mpsc::channel();
        Mpsc {
            tx: tx,
            rx: rx
        }
    }
}

#[no_mangle]
pub unsafe fn ice_ext_mpsc_create() -> *mut Mpsc {
    Box::into_raw(Box::new(Mpsc::new()))
}

#[no_mangle]
pub unsafe fn ice_ext_mpsc_destroy(mpsc: *mut Mpsc) {
    Box::from_raw(mpsc);
}

#[no_mangle]
pub unsafe fn ice_ext_mpsc_create_sender(mpsc: *mut Mpsc) -> *mut Sender {
    let mpsc = &*mpsc;
    Box::into_raw(Box::new(Sender {
        tx: mpsc.tx.clone()
    }))
}

#[no_mangle]
pub unsafe fn ice_ext_mpsc_destroy_sender(sender: *mut Sender) {
    Box::from_raw(sender);
}

#[no_mangle]
pub unsafe fn ice_ext_mpsc_sender_clone(sender: *mut Sender) -> *mut Sender {
    let sender = &*sender;
    Box::into_raw(Box::new(sender.clone()))
}

#[no_mangle]
pub unsafe fn ice_ext_mpsc_sender_write(sender: *mut Sender, data: *mut UnknownData) {
    let sender = &*sender;
    sender.tx.send(data).unwrap();
}

#[no_mangle]
pub unsafe fn ice_ext_mpsc_read(mpsc: *mut Mpsc) -> *mut UnknownData {
    let mpsc = &*mpsc;
    mpsc.rx.recv().unwrap()
}
