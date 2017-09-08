use std;
use futures;
pub mod api;

pub struct ReadStream {
    receiver: Option<futures::sync::mpsc::Receiver<Vec<u8>>>
}

impl From<futures::sync::mpsc::Receiver<Vec<u8>>> for ReadStream {
    fn from(other: futures::sync::mpsc::Receiver<Vec<u8>>) -> ReadStream {
        ReadStream {
            receiver: Some(other)
        }
    }
}

impl ReadStream {
    pub fn take_receiver(&mut self) -> futures::sync::mpsc::Receiver<Vec<u8>> {
        std::mem::replace(&mut self.receiver, None).unwrap()
    }
}
