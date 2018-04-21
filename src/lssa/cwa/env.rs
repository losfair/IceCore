use super::super::namespace::InvokeContext;
use wasm_core::value::Value;
use super::ErrorCode;

decl_namespace!(
    EnvNs,
    "env",
    EnvImpl,
    get
);

pub struct EnvImpl;

impl EnvImpl {
    pub fn get(&self, mut ctx: InvokeContext) -> Option<Value> {
        let key = ctx.extract_str(0, 1);
        let app = ctx.app.upgrade().unwrap();

        let val = app.config.env.get(key);

        Some(match val {
            Some(v) => {
                let buf = ctx.extract_bytes_mut(2, 3);
                let bytes = v.as_bytes();

                if bytes.len() > buf.len() {
                    // The buffer is not big enough to hold the data to return.
                    // Caller should check this.
                    Value::I32(bytes.len() as _)
                } else {
                    buf[0..bytes.len()].copy_from_slice(bytes);
                    Value::I32(bytes.len() as _)
                }
            },
            None => Value::I32(ErrorCode::NotFoundError.into())
        })
    }
}
