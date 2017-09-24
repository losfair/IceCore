use hyper;
use hyper::server::Response;
use futures;
use futures::future::Future;
use std;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use futures::Sink;
use tokio_core;
use futures::sync::oneshot;
use etag::Etag;

use ice_server;
use logging;

pub struct WorkerControlMessage {
    path: String,
    etag: Option<String>,
    metadata_tx: oneshot::Sender<Metadata>,
    data_tx: futures::sync::mpsc::Sender<Result<hyper::Chunk, hyper::Error>>
}

/*
#[derive(Debug)]
struct FileCacheItem {
    data: Vec<u8>,
    metadata: std::fs::Metadata
}
*/

#[derive(Debug)]
enum Metadata {
    IoError(std::io::Error),
    NotModified,
    //CacheHit(Arc<FileCacheItem>),
    Ok(std::fs::Metadata, String /* ETag */)
}

pub fn fetch_raw_unchecked(_: &ice_server::Context, local_ctx: &ice_server::LocalContext, mut resp: Response, p: &str, etag: Option<String>) -> Box<Future<Item = Response, Error = String>> {
    let (data_tx, data_rx) = futures::sync::mpsc::channel(64);
    let (metadata_tx, metadata_rx) = oneshot::channel();

    local_ctx.static_file_worker_control_tx.send(WorkerControlMessage {
        path: p.to_string(),
        etag: etag,
        metadata_tx: metadata_tx,
        data_tx: data_tx
    }).unwrap();

    let suffix = p.split(".").last().unwrap();
    let content_type = match suffix {
        "htm" | "html" => "text/html",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "js" => "application/javascript",
        "mp3" => "audio/mpeg",
        "css" => "text/css",
        _ => "application/octet-stream"
    };

    {
        let headers = resp.headers_mut();
        if let None = headers.get::<hyper::header::ContentType>() {
            headers.set_raw("Content-Type", content_type);
        }
    }

    Box::new(metadata_rx.map(move |m| {
        match m {
            Metadata::IoError(e) => resp.with_status(match e.kind() {
                std::io::ErrorKind::NotFound => hyper::StatusCode::NotFound,
                std::io::ErrorKind::PermissionDenied => hyper::StatusCode::Forbidden,
                _ => hyper::StatusCode::InternalServerError
            }),
            Metadata::NotModified => resp.with_status(hyper::StatusCode::NotModified),
            /*Metadata::CacheHit(c) => {
                resp
                .with_header(hyper::header::ContentLength(c.metadata.len()))
                .with_header(hyper::header::ETag(hyper::header::EntityTag::new(true, c.metadata.etag())))
                .with_header(hyper::header::Expires((std::time::SystemTime::now() + std::time::Duration::from_secs(300)).into()))
                .with_body(hyper::Chunk::from(c.data.clone()))
            },*/
            Metadata::Ok(m, etag) => {
                resp
                .with_header(hyper::header::ContentLength(m.len()))
                .with_header(hyper::header::ETag(hyper::header::EntityTag::new(true, etag)))
                .with_header(hyper::header::Expires((std::time::SystemTime::now() + std::time::Duration::from_secs(300)).into()))
                .with_body(data_rx)
            }
        }
    }).map_err(|_| "Error".to_string()))
}

pub fn worker(_: Arc<ice_server::Context>, remote_handle: tokio_core::reactor::Remote, control_rx: std::sync::mpsc::Receiver<WorkerControlMessage>) {
    let mut warning_showed = false;
    let logger = logging::Logger::new("static_file::worker");

    loop {
        let msg = control_rx.recv().unwrap();
        let remote_handle_cloned = remote_handle.clone();
        let data_tx = msg.data_tx.clone();
        let path = msg.path.clone();

        if !warning_showed {
            logger.log(logging::Message::Warning("Please use a reverse proxy to serve static files in production.".to_string()));
            warning_showed = true;
        }

        //let logger = logger.clone();

        (move || {
            let remote_handle = remote_handle_cloned;

            let m = match std::fs::metadata(path.as_str()) {
                Ok(v) => v,
                Err(e) => return msg.metadata_tx.send(Metadata::IoError(e)).unwrap()
            };

            let current_etag = m.etag();

            if let Some(v) = msg.etag.clone() {
                if current_etag == v {
                    return msg.metadata_tx.send(Metadata::NotModified).unwrap();
                }
            }

            //logger.log(logging::Message::Info(format!("Opening file")));

            let mut f = match File::open(path.as_str()) {
                Ok(f) => f,
                Err(e) => return msg.metadata_tx.send(Metadata::IoError(e)).unwrap()
            };

            msg.metadata_tx.send(Metadata::Ok(m.clone(), current_etag)).unwrap();

            let reader = move || {
                let mut data_tx = data_tx.clone();
                //let logger = logging::Logger::new("static_file::worker::reader");

                loop {
                    let mut buf = [0; 32768];
                    let len = match f.read(&mut buf[..]) {
                        Ok(v) => v,
                        Err(_) => break
                    };
                    if len == 0 {
                        break;
                    }
                    let data_tx = data_tx.clone();
                    let buf = buf[0..len].to_vec();

                    remote_handle.clone().spawn(move |_| {
                        data_tx.send(Ok(hyper::Chunk::from(buf))).map_err(|_| ()).map(|_| ())
                    });
                }

                remote_handle.spawn(move |_| {
                    data_tx.close().unwrap();
                    Ok(())
                });
            };

            std::thread::spawn(reader);
        })();
    }
}
