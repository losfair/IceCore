use super::super::namespace::InvokeContext;
use wasm_core::value::Value;

decl_namespace!(
    RuntimeNs,
    "runtime",
    RuntimeImpl,
    spec_major,
    spec_minor,
    name
);

pub struct RuntimeImpl;

impl RuntimeImpl {
    pub fn spec_major(&self, _: InvokeContext) -> Option<Value> {
        Some(Value::I32(super::MAJOR_VERSION))
    }

    pub fn spec_minor(&self, _: InvokeContext) -> Option<Value> {
        Some(Value::I32(super::MINOR_VERSION))
    }

    pub fn name(&self, mut ctx: InvokeContext) -> Option<Value> {
        let out = ctx.extract_bytes_mut(0, 1);
        let name = "Ice".as_bytes();

        if out.len() < name.len() {
            return Some(Value::I32(super::ErrorCode::InvalidArgumentError.into()));
        }

        out[0..name.len()].copy_from_slice(name);
        Some(Value::I32(name.len() as i32))
    }
}
