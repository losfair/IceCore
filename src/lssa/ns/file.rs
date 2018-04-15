use std::cell::RefCell;

use slab::Slab;
use std::fs::{File, OpenOptions};

use super::super::namespace::InvokeContext;
use super::super::error::ErrorCode;
use wasm_core::value::Value;
use config::AppPermission;

decl_namespace!(
    FileNs,
    "file",
    FileImpl,
    open,
    close,
    read,
    write,
    flush,
    seek
);

pub struct FileImpl {
    handles: RefCell<Slab<File>>
}

impl FileImpl {
    pub fn new() -> FileImpl {
        FileImpl {
            handles: RefCell::new(Slab::new())
        }
    }

    pub fn open(&self, ctx: InvokeContext) -> Option<Value> {
        let path = ctx.extract_str(0, 1);
        let mode = ctx.extract_str(2, 3);
        let mut opt = OpenOptions::new();

        let app = ctx.app.upgrade().unwrap();
        match app.check_permission(&AppPermission::FileOpenReadOnlyAny)
            .or_else(|_| app.check_permission(&AppPermission::FileOpenReadWriteAny)) {
                Ok(_) => {},
                Err(_) => {
                    derror!(
                        logger!(&app.name),
                        "FileOpenReadOnlyAny or FileOpenReadWriteAny permissions are required"
                    );
                    return Some(ErrorCode::PermissionDenied.to_ret());
                }
            }

        let mut need_write = false;

        for ch in mode.chars() {
            match ch {
                'r' => {
                    opt.read(true);
                },
                'w' => {
                    opt.write(true);
                    need_write = true;
                },
                'a' => {
                    opt.append(true);
                    need_write = true;
                },
                't' => {
                    opt.truncate(true);
                    need_write = true;
                },
                'c' => {
                    opt.create(true);
                    need_write = true;
                },
                'n' => {
                    opt.create_new(true);
                    need_write = true;
                },
                _ => return Some(ErrorCode::InvalidInput.to_ret())
            }
        }

        if need_write {
            match app.check_permission(&AppPermission::FileOpenReadWriteAny) {
                Ok(_) => {},
                Err(_) => {
                    derror!(
                        logger!(&app.name),
                        "FileOpenReadWriteAny permission is required"
                    );
                    return Some(ErrorCode::PermissionDenied.to_ret());
                }
            }
        }

        let f = match opt.open(path) {
            Ok(v) => v,
            Err(e) => return Some(ErrorCode::from(e.kind()).to_ret())
        };

        let id = self.handles.borrow_mut().insert(f);

        Some(Value::I32(id as i32))
    }

    pub fn close(&self, ctx: InvokeContext) -> Option<Value> {
        let id = ctx.args[0].get_i32().unwrap() as usize;
        self.handles.borrow_mut().remove(id);
        None
    }

    pub fn read(&self, mut ctx: InvokeContext) -> Option<Value> {
        use std::io::Read;

        let id = ctx.args[0].get_i32().unwrap() as usize;
        let buf = ctx.extract_bytes_mut(1, 2);

        let mut handles = self.handles.borrow_mut();
        let file = handles.get_mut(id).unwrap();

        Some(match file.read(buf) {
            Ok(n) => Value::I32(n as i32),
            Err(e) => ErrorCode::from(e.kind()).to_ret()
        })
    }

    pub fn write(&self, ctx: InvokeContext) -> Option<Value> {
        use std::io::Write;

        let id = ctx.args[0].get_i32().unwrap() as usize;
        let buf = ctx.extract_bytes(1, 2);

        let mut handles = self.handles.borrow_mut();
        let file = handles.get_mut(id).unwrap();

        Some(match file.write(buf) {
            Ok(n) => Value::I32(n as i32),
            Err(e) => ErrorCode::from(e.kind()).to_ret()
        })
    }

    pub fn flush(&self, ctx: InvokeContext) -> Option<Value> {
        use std::io::Write;

        let id = ctx.args[0].get_i32().unwrap() as usize;

        let mut handles = self.handles.borrow_mut();
        let file = handles.get_mut(id).unwrap();

        Some(match file.flush() {
            Ok(()) => ErrorCode::Success.to_ret(),
            Err(e) => ErrorCode::from(e.kind()).to_ret()
        })
    }

    pub fn seek(&self, ctx: InvokeContext) -> Option<Value> {
        use std::io::{Seek, SeekFrom};

        let id = ctx.args[0].get_i32().unwrap() as usize;
        let from = ctx.args[1].get_i32().unwrap();
        let offset = ctx.args[2].get_i64().unwrap();

        let from = match from {
            0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::End(offset),
            2 => SeekFrom::Current(offset),
            _ => return Some(
                Value::I64(ErrorCode::InvalidInput.to_i32() as i64)
            )
        };

        let mut handles = self.handles.borrow_mut();
        let file = handles.get_mut(id).unwrap();

        Some(Value::I64(match file.seek(from) {
            Ok(v) => v as i64,
            Err(e) => ErrorCode::from(e.kind()).to_i32() as i64
        }))
    }
}
