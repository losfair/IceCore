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

use ice_server;

#[derive(Clone)]
pub struct WorkerControlMessage {
    pub path: String,
    pub data_tx: futures::sync::mpsc::Sender<Result<hyper::Chunk, hyper::Error>>
}

pub fn fetch(ctx: &ice_server::Context, p: &str, dir: &str) -> Box<Future<Item = Response, Error = String>> {
    if !p.starts_with("/") || p.contains("..") { // TODO: Is this really safe ?
        println!("[static_file::fetch] Blocked: {}", p);
        return futures::future::err("Invalid path".to_string()).boxed();
    }

    let (data_tx, data_rx) = futures::sync::mpsc::channel(4096);
    ctx.static_file_worker_control_tx.send(WorkerControlMessage {
        path: dir.to_string() + p,
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

    let mut headers = hyper::header::Headers::new();
    headers.set_raw("Content-Type", content_type);

    Box::new(futures::future::ok(Response::new().with_headers(headers).with_body(data_rx)))
}

pub fn worker(remote_handle: tokio_core::reactor::Remote, control_rx: std::sync::mpsc::Receiver<WorkerControlMessage>) {
    loop {
        let mut msg = control_rx.recv().unwrap();
        let msg_cloned = msg.clone();
        let remote_handle_cloned = remote_handle.clone();


        let transform_err = |e: &str| {
            //Err(hyper::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))
            Ok(hyper::Chunk::from(Vec::new()))
        };

        (move || {
            let msg = msg_cloned;
            let data_tx = msg.data_tx.clone();
            let remote_handle = remote_handle_cloned;

            let mut f = match File::open(msg.path.as_str()) {
                Ok(f) => f,
                Err(e) => {
                    let data_tx = data_tx.clone();
                    remote_handle.clone().spawn(move |_| {
                        data_tx.send(transform_err(e.description())).map_err(|_| ()).map(|_| ())
                    });
                    return;
                }
            };
            loop {
                let mut buf = [0; 32768];
                let len = match f.read(&mut buf[..]) {
                    Ok(v) => v,
                    Err(e) => {
                        let data_tx = data_tx.clone();
                        remote_handle.clone().spawn(move |_| {
                            data_tx.send(transform_err(e.description())).map_err(|_| ()).map(|_| ())
                        });
                        return;
                    }
                };
                if len == 0 {
                    break;
                }
                let data_tx = data_tx.clone();
                remote_handle.clone().spawn(move |_| {
                    data_tx.send(Ok(hyper::Chunk::from(buf[0..len].to_vec()))).map_err(|_| ()).map(|_| ())
                });
            }
        })();
        remote_handle.spawn(move |_| {
            msg.data_tx.close().unwrap();
            Ok(())
        });
    }
}
