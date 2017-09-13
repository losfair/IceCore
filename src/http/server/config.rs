use std::net::SocketAddr;

#[derive(Clone)]
pub struct HttpServerConfig {
    pub listen_addr: Option<SocketAddr>,
    pub num_executors: usize
}

impl HttpServerConfig {
    pub fn new() -> HttpServerConfig {
        HttpServerConfig {
            listen_addr: None,
            num_executors: 1
        }
    }
}
