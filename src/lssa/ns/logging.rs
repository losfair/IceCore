use super::super::namespace::InvokeContext;
use wasm_core::value::Value;

decl_namespace!(
    LoggingNs,
    "logging",
    LoggingImpl,
    info,
    warning
);

pub struct LoggingImpl;

impl LoggingImpl {
    pub fn info(&self, ctx: InvokeContext) -> Option<Value> {
        let text = ctx.extract_str(0, 1);
        let app = ctx.app.upgrade().unwrap();

        dinfo!(logger!(&app.name), "{}", text);
        None
    }

    pub fn warning(&self, ctx: InvokeContext) -> Option<Value> {
        let text = ctx.extract_str(0, 1);
        let app = ctx.app.upgrade().unwrap();

        dwarning!(logger!(&app.name), "{}", text);
        None
    }
}
