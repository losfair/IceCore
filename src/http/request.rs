use std::ops::Deref;
use std::str::FromStr;
use hyper;

pub struct Request {
    method: Option<hyper::Method>,
    uri: Option<hyper::Uri>,
    body: Option<hyper::Chunk>
}

impl Request {
    pub fn new() -> Request {
        Request {
            method: None,
            uri: None,
            body: None
        }
    }

    pub fn into_hyper_request(self) -> Option<hyper::Request> {
        let mut ret = hyper::Request::new(
            match self.method {
                Some(v) => v,
                None => return None
            },
            match self.uri {
                Some(v) => v,
                None => return None
            }
        );
        if let Some(body) = self.body {
            ret.set_body(body);
        }
        Some(ret)
    }

    pub fn set_method(&mut self, m: &str) -> bool {
        self.method = match hyper::Method::from_str(m) {
            Ok(v) => Some(v),
            Err(_) => return false
        };
        return true;
    }

    pub fn set_body(&mut self, data: &[u8]) {
        self.body = Some(data.to_vec().into());
    }
}
