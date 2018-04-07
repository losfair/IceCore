use tokio;
use std::net::SocketAddr;
use futures::Future;
use std::sync::{Arc, Mutex};

pub struct TcpConnection {
    stream: Arc<Mutex<Option<tokio::net::TcpStream>>>
}

pub fn listen(addr: &str) -> tokio::net::Incoming {
    let saddr: SocketAddr = addr.parse().unwrap();
    let listener = tokio::net::TcpListener::bind(&saddr).unwrap();

    listener.incoming()
}

impl TcpConnection {
    pub fn new(stream: tokio::net::TcpStream) -> TcpConnection {
        TcpConnection {
            stream: Arc::new(Mutex::new(Some(stream)))
        }
    }

    pub fn write(&self, data: Vec<u8>) -> impl Future<Item = (), Error = tokio::io::Error> {
        let stream_box = self.stream.clone();

        let s = stream_box.lock().unwrap().take().unwrap();

        tokio::io::write_all(
            s,
            data
        ).map(move |(a, _)| {
            *stream_box.lock().unwrap() = Some(a);
        })
    }
}
