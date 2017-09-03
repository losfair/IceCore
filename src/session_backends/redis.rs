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
use redis::Commands;
use session_storage::*;
use uuid::Uuid;

#[derive(Clone)]
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
    GetSession(String),
    Get(String),
    Set(String, String),
    Remove(String)
}

#[derive(Debug)]
enum OpResponse {
    CreateSession(Session),
    GetSession(Option<Session>),
    Get(Option<String>),
    Set,
    Remove
}

pub struct RedisStorageImpl {
    remote_handle: tokio_core::reactor::Remote,
    client: redis::Client,
    op_request_receiver: Mutex<Option<futures::sync::mpsc::Receiver<OpRequestMessage>>>,
    op_request_channel: futures::sync::mpsc::Sender<OpRequestMessage>
}

#[derive(Clone)]
pub struct RedisSession {
    id: String,
    storage: Arc<RedisStorageImpl>
}

impl SessionProvider for RedisSession {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get(&self, key: &str) -> Option<String> {
        let key = key.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        let self_cloned = self.clone();

        self.storage.remote_handle.spawn(move |_| {
            let tx_cloned = tx.clone();
            self_cloned.get_async(key.as_str())
                .map(move |v| {
                    tx_cloned.send(v);
                    ()
                })
                .map_err(move |e| {
                    tx.send(None);
                    ()
                })
        });
        rx.recv().unwrap()
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
        let key = key.to_string();
        let value = value.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        let self_cloned = self.clone();

        self.storage.remote_handle.spawn(move |_| {
            let tx_cloned = tx.clone();
            self_cloned.set_async(key.as_str(), value.as_str())
                .map(move |_| {
                    tx_cloned.send(());
                    ()
                })
                .map_err(move |_| {
                    tx.send(());
                    ()
                })
        });
        rx.recv().unwrap()
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
        let key = key.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        let self_cloned = self.clone();

        self.storage.remote_handle.spawn(move |_| {
            let tx_cloned = tx.clone();
            self_cloned.remove_async(key.as_str())
                .map(move |_| {
                    tx_cloned.send(());
                    ()
                })
                .map_err(move |_| {
                    tx.send(());
                    ()
                })
        });
        rx.recv().unwrap()
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

impl SessionStorageProvider for RedisStorage {
    fn create_session(&self) -> Session {
        let (tx, rx) = std::sync::mpsc::channel();
        let self_cloned = self.clone();

        self.remote_handle.spawn(move |_| {
            let tx_cloned = tx.clone();
            self_cloned.create_session_async()
                .map(move |v| {
                    tx_cloned.send(Some(v));
                    ()
                })
                .map_err(move |e| {
                    tx.send(None);
                    ()
                })
        });
        rx.recv().unwrap().unwrap()
    }

    fn create_session_async(&self) -> Box<Future<Item = Session, Error = ()>> {
        let sender = self.op_request_channel.clone();
        let (resp_tx, resp_rx) = oneshot::channel();

        let msg = OpRequestMessage {
            response_channel: resp_tx,
            request: OpRequest::CreateSession,
            session_id: None
        };

        self.remote_handle.spawn(move |_| {
            sender.send(msg).map(|_| ()).map_err(|_| ())
        });
        Box::new(resp_rx.map(|ret| {
            match ret {
                OpResponse::CreateSession(v) => v,
                _ => panic!()
            }
        }).map_err(|e| {
            println!("{:?}", e);
            ()
        }))
    }

    fn get_session(&self, id: &str) -> Option<Session> {
        let id = id.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        let self_cloned = self.clone();

        self.remote_handle.spawn(move |_| {
            let tx_cloned = tx.clone();
            self_cloned.get_session_async(id.as_str())
                .map(move |v| {
                    tx_cloned.send(v);
                    ()
                })
                .map_err(move |e| {
                    tx.send(None);
                    ()
                })
        });
        rx.recv().unwrap()
    }

    fn get_session_async(&self, id: &str) -> Box<Future<Item = Option<Session>, Error = ()>> {
        let sender = self.op_request_channel.clone();
        let (resp_tx, resp_rx) = oneshot::channel();

        let msg = OpRequestMessage {
            response_channel: resp_tx,
            request: OpRequest::GetSession(id.to_string()),
            session_id: None
        };

        self.remote_handle.spawn(move |_| {
            sender.send(msg).map(|_| ()).map_err(|_| ())
        });
        Box::new(resp_rx.map(|ret| {
            match ret {
                OpResponse::GetSession(v) => v,
                _ => panic!()
            }
        }).map_err(|e| {
            println!("{:?}", e);
            ()
        }))
    }

    fn start(&self) {
        RedisStorage::start_worker(self.inner.clone());
    }
}

impl RedisStorage {
    pub fn new(remote_handle: tokio_core::reactor::Remote, conn_str: &str) -> RedisStorage {
        let (op_tx, op_rx) = futures::sync::mpsc::channel(1024);
        let inner = Arc::new(RedisStorageImpl {
            remote_handle: remote_handle,
            client: redis::Client::open(conn_str).unwrap(),
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
                            let conn = me.client.get_connection().unwrap();
                            let sess = Arc::new(RedisSession {
                                id: Uuid::new_v4().to_string(),
                                storage: me.clone()
                            });
                            let _: () = conn.set("ice-session-".to_string() + sess.id.as_str(), true).unwrap();
                            OpResponse::CreateSession(sess.into())
                        },
                        OpRequest::GetSession(id) => {
                            let conn = me.client.get_connection().unwrap();
                            let ok: Option<String> = conn.get("ice-session-".to_string() + id.as_str()).unwrap();
                            OpResponse::GetSession(
                                match ok {
                                    Some(_) => Some(
                                        Arc::new(RedisSession {
                                            id: id,
                                            storage: me.clone()
                                        }).into()
                                    ),
                                    None => None
                                }
                            )
                        },
                        OpRequest::Get(key) => {
                            let conn = me.client.get_connection().unwrap();
                            let value: Option<String> = conn.get(
                                get_session_prefix(req.session_id.as_ref().unwrap().as_str())
                                    + key.as_str()
                            ).unwrap();
                            OpResponse::Get(value)
                        },
                        OpRequest::Set(key, value) => {
                            let conn = me.client.get_connection().unwrap();
                            let _: () = conn.set(
                                get_session_prefix(req.session_id.as_ref().unwrap().as_str())
                                    + key.as_str(),
                                value
                            ).unwrap();
                            OpResponse::Set
                        },
                        OpRequest::Remove(key) => {
                            let conn = me.client.get_connection().unwrap();
                            let _: () = conn.set(
                                get_session_prefix(req.session_id.as_ref().unwrap().as_str())
                                    + key.as_str(),
                                None as Option<String>
                            ).unwrap();
                            OpResponse::Remove
                        }
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

fn get_session_prefix(id: &str) -> String {
    "ice-session-".to_string() + id + "-"
}
