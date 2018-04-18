use super::super::namespace::InvokeContext;
use wasm_core::value::Value;

decl_namespace!(
    LogNs,
    "log",
    LogImpl,
    write
);

pub struct LogImpl;

impl LogImpl {
    pub fn write(&self, ctx: InvokeContext) -> Option<Value> {
        let app = ctx.app.upgrade().unwrap();

        let level = ctx.args[0].get_i32().unwrap();
        let text = ctx.extract_str(1, 2);

        use logging::Level;

        let level = match level {
            1 => Level::Error,
            3 => Level::Warning,
            6 => Level::Info,
            _ => Level::Info
        };

        let logger = ::logging::Logger::new(&app.name);
        logger.log(level, text);

        None
    }
}
