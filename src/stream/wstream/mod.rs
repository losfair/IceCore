use std::sync::Mutex;
use std::ops::Deref;
use futures;
pub mod api;

pub struct WriteStream {
    sender: Mutex<futures::sync::mpsc::Sender<Vec<u8>>>
}

impl From<futures::sync::mpsc::Sender<Vec<u8>>> for WriteStream {
    fn from(other: futures::sync::mpsc::Sender<Vec<u8>>) -> WriteStream {
        WriteStream {
            sender: Mutex::new(other)
        }
    }
}

impl Deref for WriteStream {
    type Target = Mutex<futures::sync::mpsc::Sender<Vec<u8>>>;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}
