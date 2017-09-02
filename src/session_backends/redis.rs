use std;
use std::sync::{Arc, Mutex};
use std::ops::Deref;
use tokio_core;
use futures;
use futures::Sink;
use futures::{future, Future};
use futures::sync::oneshot;
use futures::Stream;
use redis;
use session_storage::*;
use uuid::Uuid;

pub struct RedisStorage {
    inner: Arc<RedisStorageImpl>
}

impl Deref for RedisStorage {
    type Target = RedisStorageImpl;
    fn deref(&self) -> &RedisStorageImpl {
        &*self.inner
    }
}

struct OpRequestMessage {
    response_channel: oneshot::Sender<OpResponse>,
    request: OpRequest,
    session_id: Option<String>
}

enum OpRequest {
    CreateSession,
    Get(String),
    Set(String, String),
    Remove(String)
}

#[derive(Debug)]
enum OpResponse {
    CreateSession(Session),
    Get(Option<String>),
    Set,
    Remove
}

pub struct RedisStorageImpl {
    remote_handle: tokio_core::reactor::Remote,
    op_request_receiver: Mutex<Option<futures::sync::mpsc::Receiver<OpRequestMessage>>>,
    op_request_channel: futures::sync::mpsc::Sender<OpRequestMessage>
}

pub struct RedisSession {
    id: String,
    storage: Arc<RedisStorageImpl>
}

impl SessionProvider for RedisSession {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get(&self, key: &str) -> Option<String> {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        match core.run(self.get_async(key)) {
            Ok(v) => v,
            Err(e) => None
        }
    }

    fn get_async(&self, key: &str) -> Box<Future<Item = Option<String>, Error = ()>> {
        let sender = self.storage.op_request_channel.clone();
        let (resp_tx, resp_rx) = oneshot::channel();

        let msg = OpRequestMessage {
            response_channel: resp_tx,
            request: OpRequest::Get(key.to_string()),
            session_id: Some(self.id.clone())
        };

        self.storage.remote_handle.spawn(move |_| {
            sender.send(msg).map(|_| ()).map_err(|_| ())
        });
        Box::new(resp_rx.map(|v| {
            match v {
                OpResponse::Get(v) => v,
                _ => None
            }
        }).map_err(|e| {
            println!("{:?}", e);
            ()
        }))
    }

    fn set(&self, key: &str, value: &str) {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        match core.run(self.set_async(key, value)) {
            _ => ()
        }
    }

    fn set_async(&self, key: &str, value: &str) -> Box<Future<Item = (), Error = ()>> {
        let sender = self.storage.op_request_channel.clone();
        let (resp_tx, resp_rx) = oneshot::channel();

        let msg = OpRequestMessage {
            response_channel: resp_tx,
            request: OpRequest::Set(key.to_string(), value.to_string()),
            session_id: Some(self.id.clone())
        };

        self.storage.remote_handle.spawn(move |_| {
            sender.send(msg).map(|_| ()).map_err(|_| ())
        });
        Box::new(resp_rx.map(|_| ()).map_err(|e| {
            println!("{:?}", e);
            ()
        }))
    }

    fn remove(&self, key: &str) {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        match core.run(self.remove_async(key)) {
            _ => ()
        }
    }

    fn remove_async(&self, key: &str) -> Box<Future<Item = (), Error = ()>> {
        let sender = self.storage.op_request_channel.clone();
        let (resp_tx, resp_rx) = oneshot::channel();

        let msg = OpRequestMessage {
            response_channel: resp_tx,
            request: OpRequest::Remove(key.to_string()),
            session_id: Some(self.id.clone())
        };

        self.storage.remote_handle.spawn(move |_| {
            sender.send(msg).map(|_| ()).map_err(|_| ())
        });
        Box::new(resp_rx.map(|_| ()).map_err(|e| {
            println!("{:?}", e);
            ()
        }))
    }
}

/*
impl SessionStorageProvider for RedisStorage {
    
}*/

impl RedisStorage {
    pub fn new(remote_handle: tokio_core::reactor::Remote, conn_str: &str) -> RedisStorage {
        let (op_tx, op_rx) = futures::sync::mpsc::channel(1024);
        let inner = Arc::new(RedisStorageImpl {
            remote_handle: remote_handle,
            op_request_receiver: Mutex::new(Some(op_rx)),
            op_request_channel: op_tx
        });
        RedisStorage {
            inner: inner
        }
    }

    fn start_worker(me: Arc<RedisStorageImpl>) {
        let op_rx = std::mem::replace(
            &mut *me.op_request_receiver.lock().unwrap(),
            None
        ).unwrap();
        let me_cloned = me.clone();

        me.remote_handle.spawn(move |_| {
            op_rx.for_each(move |req| {
                let me = me_cloned.clone();
                std::thread::spawn(move || {
                    let resp = match req.request {
                        OpRequest::CreateSession => {
                            let sess = Arc::new(RedisSession {
                                id: Uuid::new_v4().to_string(),
                                storage: me.clone()
                            });
                            OpResponse::CreateSession(sess.into())
                        },
                        OpRequest::Get(key) => OpResponse::Get(None),
                        OpRequest::Set(key, value) => OpResponse::Set,
                        OpRequest::Remove(key) => OpResponse::Remove
                    };
                    req.response_channel.send(resp).unwrap();
                });

                Ok(())
            }).map(|_| ()).map_err(|e| {
                println!("{:?}", e);
                ()
            })
        });
    }
}
