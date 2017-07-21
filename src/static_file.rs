use hyper;
use hyper::server::Response;
use futures;
use futures::future::Future;
use std;
use std::fs::File;
use std::error::Error;
use std::io::Read;
use futures::Sink;
use tokio_core;
use futures::Stream;
use futures::sync::oneshot;

use ice_server;

pub struct WorkerControlMessage {
    path: String,
    metadata_tx: oneshot::Sender<Metadata>,
    data_tx: futures::sync::mpsc::Sender<Result<hyper::Chunk, hyper::Error>>
}

#[derive(Debug)]
enum Metadata {
    IoError(std::io::Error),
    Ok(std::fs::Metadata)
}

pub fn fetch(ctx: &ice_server::Context, p: &str, dir: &str) -> Box<Future<Item = Response, Error = String>> {
    if !p.starts_with("/") || p.contains("..") { // TODO: Is this really safe ?
        println!("[static_file::fetch] Blocked: {}", p);
        return futures::future::err("Invalid path".to_string()).boxed();
    }

    fetch_raw_unchecked(&ctx, Response::new(), (dir.to_string() + p).as_str())
}

pub fn fetch_raw_unchecked(ctx: &ice_server::Context, mut resp: Response, p: &str) -> Box<Future<Item = Response, Error = String>> {
    let (data_tx, data_rx) = futures::sync::mpsc::channel(64);
    let (metadata_tx, metadata_rx) = oneshot::channel();

    ctx.static_file_worker_control_tx.send(WorkerControlMessage {
        path: p.to_string(),
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
        let mut headers = resp.headers_mut();
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
            Metadata::Ok(m) => resp.with_header(hyper::header::ContentLength(m.len())).with_body(data_rx)
        }
    }).map_err(|_| "Error".to_string()))
}

pub fn worker(remote_handle: tokio_core::reactor::Remote, control_rx: std::sync::mpsc::Receiver<WorkerControlMessage>) {
    loop {
        let mut msg = control_rx.recv().unwrap();
        let remote_handle_cloned = remote_handle.clone();
        let data_tx = msg.data_tx.clone();
        let path = msg.path.clone();

        (move || {
            let remote_handle = remote_handle_cloned;

            let m = match std::fs::metadata(path.as_str()) {
                Ok(v) => v,
                Err(e) => return msg.metadata_tx.send(Metadata::IoError(e)).unwrap()
            };

            let mut f = match File::open(path.as_str()) {
                Ok(f) => f,
                Err(e) => return msg.metadata_tx.send(Metadata::IoError(e)).unwrap()
            };

            msg.metadata_tx.send(Metadata::Ok(m.clone())).unwrap();

            let reader = move || {
                let mut data_tx = data_tx.clone();
                loop {
                    let mut buf = [0; 32768];
                    let len = match f.read(&mut buf[..]) {
                        Ok(v) => v,
                        Err(e) => break
                    };
                    if len == 0 {
                        break;
                    }
                    let data_tx = data_tx.clone();
                    remote_handle.clone().spawn(move |_| {
                        data_tx.send(Ok(hyper::Chunk::from(buf[0..len].to_vec()))).map_err(|_| ()).map(|_| ())
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
