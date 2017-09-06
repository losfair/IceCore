use std;
use std::error::Error;
use storage::kv::KVStorage;
use storage::error::StorageError;
use threadpool::ThreadPool;
use r2d2;
use r2d2_redis::RedisConnectionManager;
use futures;
use futures::sync::oneshot;
use futures::Future;
use redis::Commands;
use redis::RedisResult;

pub struct RedisStorage {
    op_tx: std::sync::mpsc::Sender<Op>
}

struct Op {
    cmd: Command,
    result_ch: Option<oneshot::Sender<OpResult>>
}

impl Op {
    fn run(storage: &RedisStorage, cmd: Command) -> Box<Future<Item = OpResult, Error = String>> {
        let (tx, rx) = oneshot::channel();

        let op = Op {
            cmd: cmd,
            result_ch: Some(tx)
        };

        storage.op_tx.clone().send(op).unwrap();
        Box::new(rx.map_err(|e| e.description().to_string()))
    }
}

#[derive(Debug)]
enum OpResult {
    Error(String),
    Value(Option<String>)
}

enum Command {
    Stop,
    Get(String),
    Set(String, String),
    Remove(String)
}

impl RedisStorage {
    pub fn new(conn_str: &str) -> RedisStorage {
        let conn_manager = RedisConnectionManager::new(conn_str).unwrap();
        let conn_pool = r2d2::Pool::new(std::default::Default::default(), conn_manager).unwrap();

        let (op_tx, op_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || RedisStorage::worker(conn_pool, op_rx));

        RedisStorage {
            op_tx: op_tx
        }
    }

    fn worker(
        conn_pool: r2d2::Pool<RedisConnectionManager>,
        op_rx: std::sync::mpsc::Receiver<Op>
    ) {
        let thread_pool = ThreadPool::new(16);

        loop {
            let op = op_rx.recv().unwrap();

            match op.cmd {
                Command::Stop => {
                    return;
                },
                _ => {}
            }

            let conn_pool = conn_pool.clone();

            thread_pool.execute(move || {
                let conn = conn_pool.get().unwrap();

                let result = match op.cmd {
                    Command::Get(k) => {
                        match conn.get(k.as_str()) {
                            Ok(v) => OpResult::Value(v),
                            Err(e) => OpResult::Error(e.description().to_string())
                        }
                    },
                    Command::Set(k, v) => {
                        match conn.set(k.as_str(), v.as_str()) as RedisResult<()> {
                            Ok(_) => OpResult::Value(None),
                            Err(e) => OpResult::Error(e.description().to_string())
                        }
                    },
                    Command::Remove(k) => {
                        match conn.del(k.as_str()) as RedisResult<()> {
                            Ok(_) => OpResult::Value(None),
                            Err(e) => OpResult::Error(e.description().to_string())
                        }
                    },
                    _ => OpResult::Error("Not implemented".to_string())
                };
                op.result_ch.unwrap().send(result).unwrap();
            });
        }
    }
}

impl Drop for RedisStorage {
    fn drop(&mut self) {
        self.op_tx.send(Op {
            cmd: Command::Stop,
            result_ch: None
        }).unwrap();
    }
}

impl KVStorage for RedisStorage {
    fn get(&self, k: &str) -> Box<Future<Item = Option<String>, Error = StorageError>> {
        Box::new(Op::run(self, Command::Get(k.to_string()))
            .map(|v| {
                if let OpResult::Value(v) = v {
                    v
                } else {
                    None
                }
            })
            .map_err(|e| StorageError::Other(e)))
    }

    fn set(&self, k: &str, v: &str) -> Box<Future<Item = (), Error = StorageError>> {
        Box::new(Op::run(self, Command::Set(k.to_string(), v.to_string()))
            .map(|_| ())
            .map_err(|e| StorageError::Other(e)))
    }

    fn remove(&self, k: &str) -> Box<Future<Item = (), Error = StorageError>> {
        Box::new(Op::run(self, Command::Remove(k.to_string()))
            .map(|_| ())
            .map_err(|e| StorageError::Other(e)))
    }
}
