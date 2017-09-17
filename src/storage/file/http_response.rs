use std;
use std::io::Read;
use std::fs::File;
use std::error::Error;
use hyper;
use etag::Etag;
use executor;
use futures;
use futures::Future;
use futures::Sink;
use logging;

lazy_static! {
    static ref LOGGER: logging::Logger = logging::Logger::new("storage::file::http_response");
}

pub fn begin_send(req: &hyper::Request, resp: &mut hyper::Response, path: &str) -> Result<(), String> {
    let m = match std::fs::metadata(path) {
        Ok(v) => v,
        Err(e) => return Err(e.description().to_string())
    };
    let current_etag = m.etag();
    match get_etag_from_request(req) {
        Some(v) => if current_etag == v {
            resp.set_status(hyper::StatusCode::NotModified);
            resp.set_body("");
            return Ok(());
        },
        None => {}
    }

    let (tx, rx) = futures::sync::mpsc::channel(0);
    let path = path.to_string();

    executor::get_event_loop().spawn(move |_| -> Box<Future<Item = (), Error = ()>> {
        let f = match File::open(path.as_str()) {
            Ok(v) => v,
            Err(e) => {
                LOGGER.log(logging::Message::Error(e.description().to_string()));
                return Box::new(futures::future::ok(()));
            }
        };
        Box::new(do_read(f, tx).or_else(|e| {
            LOGGER.log(logging::Message::Error(e));
            Ok(())
        }))
    });
    resp.headers_mut().set(hyper::header::ETag(hyper::header::EntityTag::new(true, current_etag)));
    resp.set_body(rx);

    Ok(())
}

fn get_etag_from_request(req: &hyper::Request) -> Option<&str> {
    match req.headers().get::<hyper::header::IfNoneMatch>() {
        Some(v) => {
            match v {
                &hyper::header::IfNoneMatch::Any => None,
                &hyper::header::IfNoneMatch::Items(ref v) => {
                    if v.len() == 0 {
                        None
                    } else {
                        Some(v[0].tag())
                    }
                }
            }
        },
        None => None
    }
}

fn do_read(
    mut f: File,
    tx: futures::sync::mpsc::Sender<Result<hyper::Chunk, hyper::Error>>
) -> Box<Future<Item = (), Error = String>> {
    let mut buf = [0; 32768];
    let len = match f.read(&mut buf[..]) {
        Ok(v) => v,
        Err(e) => return Box::new(futures::future::err(e.description().to_string()))
    };
    if len == 0 {
        return Box::new(futures::future::ok(()));
    }

    let tx_cloned = tx.clone();

    Box::new(tx.send(Ok(buf[0..len].to_vec().into())).map(move |_| {
        do_read(f, tx_cloned)
    }).map_err(|e| e.description().to_string()).flatten())
}
