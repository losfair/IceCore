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
    /*
    let f = match File::open(p) {
        Ok(f) => f,
        Err(e) => return futures::future::err(e.description().to_string()).boxed()
    };
    let f = match tokio_file_unix::File::new_nb(f) {
        Ok(f) => f,
        Err(e) => return futures::future::err(e.description().to_string()).boxed()
    };
    let reader = f.into_reader(&ctx.ev_loop_handle).unwrap();
    */

    /*Box::new(tokio_io::io::read_to_end(reader, Vec::new())
    .map_err(|e| e.description().to_string())
    .map(|(_, buf)| {
        Response::new().with_body(buf)
    }))*/

    if !p.starts_with("/") || p.contains("..") { // TODO: Is this really safe ?
        return futures::future::err("Invalid path".to_string()).boxed();
    }

    let (data_tx, data_rx) = futures::sync::mpsc::channel(4096);
    ctx.static_file_worker_control_tx.send(WorkerControlMessage {
        path: dir.to_string() + p,
        data_tx: data_tx
    }).unwrap();

    Box::new(futures::future::ok(Response::new().with_body(data_rx)))

    /*reader.read_to_end().map_err(|e| e.description().to_string()).map(move |_| {
        Response::new().with_body(data)
    })*/
}

pub fn worker(remote_handle: tokio_core::reactor::Remote, control_rx: std::sync::mpsc::Receiver<WorkerControlMessage>) {
    loop {
        let mut msg = control_rx.recv().unwrap();
        let msg_cloned = msg.clone();
        let remote_handle_cloned = remote_handle.clone();


        let transform_err = |e: &str| {
            Err(hyper::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))
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
