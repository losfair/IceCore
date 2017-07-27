use hyper;
use hyper::server::Response;
use futures;
use futures::future::Future;
use std;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::sync::atomic;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
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

#[derive(Debug)]
struct FileCacheItem {
    data: Vec<u8>,
    metadata: std::fs::Metadata
}

#[derive(Debug)]
enum Metadata {
    IoError(std::io::Error),
    NotModified,
    CacheHit(Arc<FileCacheItem>),
    Ok(std::fs::Metadata, String /* ETag */)
}

pub fn fetch_raw_unchecked(ctx: &ice_server::Context, local_ctx: &ice_server::LocalContext, mut resp: Response, p: &str, etag: Option<String>) -> Box<Future<Item = Response, Error = String>> {
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
            Metadata::NotModified => resp.with_status(hyper::StatusCode::NotModified),
            Metadata::CacheHit(c) => {
                resp
                .with_header(hyper::header::ContentLength(c.metadata.len()))
                .with_header(hyper::header::ETag(hyper::header::EntityTag::new(true, c.metadata.etag())))
                .with_header(hyper::header::Expires((std::time::SystemTime::now() + std::time::Duration::from_secs(300)).into()))
                .with_body(hyper::Chunk::from(c.data.clone()))
            },
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

pub fn worker(ctx: Arc<ice_server::Context>, remote_handle: tokio_core::reactor::Remote, control_rx: std::sync::mpsc::Receiver<WorkerControlMessage>) {
    let mut warning_showed = false;
    let logger = logging::Logger::new("static_file::worker");
    let cache_prep: Rc<RefCell<VecDeque<(String, u64)>>> = Rc::new(RefCell::new(VecDeque::new()));
    let to_cache: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new(HashMap::new()));
    let cache: Arc<Mutex<HashMap<String, Arc<FileCacheItem>>>> = Arc::new(Mutex::new(HashMap::new()));
    let cache_size: Arc<atomic::AtomicIsize> = Arc::new(atomic::AtomicIsize::new(0));
    let max_cache_size = ctx.max_cache_size as isize;
    let max_queue_len = 100;

    loop {
        let msg = control_rx.recv().unwrap();
        let remote_handle_cloned = remote_handle.clone();
        let data_tx = msg.data_tx.clone();
        let path = msg.path.clone();

        if !warning_showed {
            logger.log(logging::Message::Warning("Please use a reverse proxy to serve static files in production.".to_string()));
            warning_showed = true;
        }

        let cache = cache.clone();
        let cache_size = cache_size.clone();
        let cache_prep = cache_prep.clone();
        let to_cache = to_cache.clone();

        let logger = logger.clone();

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

            {
                let mut cache = cache.lock().unwrap();
                let should_remove = match cache.get(&path) {
                    Some(v) => {
                        if v.metadata.etag() != current_etag {
                            true
                        } else {
                            return msg.metadata_tx.send(Metadata::CacheHit(v.clone())).unwrap();
                        }
                    },
                    None => false
                };
                if should_remove {
                    cache.remove(&path);
                }
                if cache_size.load(atomic::Ordering::Relaxed) + m.len() as isize <= max_cache_size {
                    let mut cache_prep = cache_prep.borrow_mut();
                    cache_prep.push_front((path.clone(), m.len()));
                    //logger.log(logging::Message::Info(format!("Loading into cache_prep: {}", path)));

                    if cache_prep.len() >= max_queue_len {
                        let mut item_counts: HashMap<String, (u64, usize)> = HashMap::new();
                        for item in cache_prep.iter() {
                            let should_insert = match item_counts.get_mut(&item.0) {
                                Some(ref mut v) => {
                                    v.1 += 1;
                                    false
                                },
                                None => true
                            };
                            if should_insert {
                                item_counts.insert(item.0.clone(), (item.1, 1));
                            }
                        }
                        let avg = item_counts.iter().map(|(k, &(size, count))| count).sum::<usize>() / item_counts.len();
                        //logger.log(logging::Message::Info(format!("Avg: {}", avg)));

                        let mut to_cache = to_cache.lock().unwrap();
                        to_cache.clear();

                        let mut current_size = cache_size.load(atomic::Ordering::Relaxed);

                        for (k, &(size, count)) in item_counts.iter().filter(|&(k, &(size, count))| count > avg) {
                            if current_size + size as isize <= max_cache_size {
                                to_cache.insert(k.clone(), true);
                                current_size += size as isize;
                            }
                        }

                        //logger.log(logging::Message::Info(format!("to_cache: {:?}", *to_cache)));

                        cache_prep.truncate(max_queue_len / 2);
                    }
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
                let logger = logging::Logger::new("static_file::worker::reader");

                let write_cache = {
                    // TODO: If `to_cache` is locked before `cache`, there may be a deadlock.
                    // A better structure is needed.
                    let cache = cache.lock().unwrap();
                    let mut to_cache = to_cache.lock().unwrap();
                    if *to_cache.get(&path).unwrap_or(&false) == true {
                        to_cache.remove(&path);
                        true
                    } else {
                        false
                    }
                };

                let mut cache_buf = Vec::new();

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

                    if write_cache {
                        cache_buf.append(&mut buf.clone());
                    }

                    remote_handle.clone().spawn(move |_| {
                        data_tx.send(Ok(hyper::Chunk::from(buf))).map_err(|_| ()).map(|_| ())
                    });
                }

                if write_cache {
                    let mut cache = cache.lock().unwrap();
                    logger.log(logging::Message::Info(format!("Writing cache: {}", path)));
                    
                    cache_size.fetch_add(cache_buf.len() as isize, atomic::Ordering::SeqCst);
                    cache.insert(path, Arc::new(FileCacheItem {
                        data: cache_buf,
                        metadata: m
                    }));
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
