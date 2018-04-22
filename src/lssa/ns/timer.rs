use super::super::namespace::{InvokeContext, MigrationProvider, Migration};
use wasm_core::value::Value;
use std::cell::Cell;
use std::rc::Rc;

use futures;
use tokio;

decl_namespace_with_migration_provider!(
    TimerNs,
    "timer",
    TimerImpl,
    TimerMigrationProvider,
    now_millis,
    set_immediate
);

pub struct TimerMigrationProvider;
impl MigrationProvider<TimerNs> for TimerMigrationProvider {
    fn start_migration(target: &TimerNs) -> Option<Migration> {
        if target.provider.pending.get() > 0 {
            None
        } else {
            target.provider.migrated.set(true);
            Some(Migration::empty())
        }
    }

    fn complete_migration(target: &TimerNs, _: &Migration) {
        if target.provider.pending.get() > 0 {
            panic!("pending > 0");
        }
    }
}

pub struct TimerImpl {
    migrated: Cell<bool>,
    pending: Rc<Cell<usize>>
}

impl TimerImpl {
    pub fn new() -> TimerImpl {
        TimerImpl {
            migrated: Cell::new(false),
            pending: Rc::new(Cell::new(0))
        }
    }
    pub fn now_millis(&self, _ctx: InvokeContext) -> Option<Value> {
        use chrono;
        let utc_time: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        Some(Value::I64(utc_time.timestamp_millis()))
    }

    pub fn set_immediate(&self, ctx: InvokeContext) -> Option<Value> {
        let app_weak = ctx.app.clone();
        let cb_target = ctx.args[0].get_i32().unwrap();
        let cb_data = ctx.args[1].get_i32().unwrap();

        if self.migrated.get() {
            panic!("migrated");
        }

        self.pending.set(self.pending.get() + 1);
        let pending = self.pending.clone();

        tokio::executor::current_thread::spawn(futures::future::lazy(move || {
            pending.set(pending.get() - 1);
            app_weak.upgrade().unwrap().invoke1(
                cb_target,
                cb_data
            );
            Ok(())
        }));

        None
    }
}
