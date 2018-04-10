use super::super::namespace::InvokeContext;
use wasm_core::value::Value;

decl_namespace!(
    TimerNs,
    "timer",
    TimerImpl,
    now_millis
);

pub struct TimerImpl;

impl TimerImpl {
    pub fn now_millis(&self, ctx: InvokeContext) -> Option<Value> {
        use chrono;
        let utc_time: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        Some(Value::I64(utc_time.timestamp_millis()))
    }
}
