use std;
use hyper;
use tokio_core;
use futures;
use futures::Sink;
use futures::Future;

type ChunkResult = Result<hyper::Chunk, hyper::Error>;
type ChunkSender = futures::sync::mpsc::Sender<ChunkResult>;
pub type ChunkReceiver = futures::sync::mpsc::Receiver<ChunkResult>;

pub struct StreamProvider {
    pub remote: tokio_core::reactor::Remote,
    pub tx: ChunkSender
}

impl StreamProvider {
    pub fn new(ev_loop: &tokio_core::reactor::Remote) -> (StreamProvider, ChunkReceiver) {
        let (tx, rx) = futures::sync::mpsc::channel(64);

        (StreamProvider {
            remote: ev_loop.clone(),
            tx: tx
        }, rx)
    }

    pub fn into_boxed(self) -> Box<StreamProvider> {
        Box::new(self)
    }

    pub unsafe fn from_raw_boxed(raw: *mut StreamProvider) -> Box<StreamProvider> {
        Box::from_raw(raw)
    }

    pub fn send_chunk(&mut self, chunk: &[u8]) {
        let chunk = hyper::Chunk::from(chunk.to_vec());
        let tx = self.tx.clone();

        self.remote.spawn(move |_| {
            tx.send(Ok(chunk)).map_err(|_| ()).map(|_| ())
        });
    }

    pub fn close(self) {
        let remote = self.remote;
        let mut tx = self.tx;

        remote.spawn(move |_| {
            tx.close().map_err(|_| ()).map(|_| ())
        });
    }
}

#[no_mangle]
pub unsafe fn ice_core_stream_provider_send_chunk(sp: *mut StreamProvider, data: *const u8, len: u32) {
    let sp = &mut *sp;
    sp.send_chunk(std::slice::from_raw_parts(data, len as usize));
}

#[no_mangle]
pub unsafe fn ice_core_destroy_stream_provider(sp: *mut StreamProvider) {
    let sp = StreamProvider::from_raw_boxed(sp);
    sp.close();
}
