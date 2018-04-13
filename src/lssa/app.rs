use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::time::SystemTime;

use chrono;

use wasm_core;
use wasm_core::jit::compiler::{Compiler, ExecutionContext};
use wasm_core::jit::runtime::RuntimeConfig;
use wasm_core::module::{Module, Type};
use container::Container;

use super::task::TaskInfo;
use super::resolver::LssaResolver;
use super::stats::AppStats;
use config::AppPermission;
use slab::Slab;

// `inner` is intended to be used internally only and this should NOT be `Clone`.
pub struct Application {
    inner: Rc<ApplicationImpl>
}

pub struct ApplicationImpl {
    pub(super) name: String,
    currently_inside: Cell<usize>,
    module: Module,
    execution: ExecutionContext,

    start_time: SystemTime,

    invoke0_fn: extern "C" fn (i64) -> i64,
    invoke1_fn: extern "C" fn (i64, i64) -> i64,
    invoke2_fn: extern "C" fn (i64, i64, i64) -> i64,
    invoke3_fn: extern "C" fn (i64, i64, i64, i64) -> i64,
    invoke4_fn: extern "C" fn (i64, i64, i64, i64, i64) -> i64,
    pub(super) container: Container,
    pub(super) tasks: RefCell<Slab<TaskInfo>>
}

struct AppInsideHandle<'a> {
    app: &'a ApplicationImpl
}

impl<'a> AppInsideHandle<'a> {
    fn new(app: &'a ApplicationImpl) -> AppInsideHandle<'a> {
        let v = app.currently_inside.get() + 1;
        app.currently_inside.set(v);

        AppInsideHandle {
            app: app
        }
    }
}

impl<'a> Drop for AppInsideHandle<'a> {
    fn drop(&mut self) {
        let v = self.app.currently_inside.get() - 1;
        self.app.currently_inside.set(v);
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub mem_default: usize,
    pub mem_max: usize,
    pub name: String
}

impl Default for AppConfig {
    fn default() -> AppConfig {
        AppConfig {
            mem_default: 32 * 65536,
            mem_max: 256 * 65536,
            name: "".into()
        }
    }
}

impl Deref for Application {
    type Target = ApplicationImpl;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl Application {
    pub fn new(
        m: Module,
        config: AppConfig,
        container: Container
    ) -> Application {
        let mut rt_config = RuntimeConfig::default();

        rt_config.mem_default = config.mem_default;
        rt_config.mem_max = config.mem_max;

        let compiler = Compiler::with_runtime_config(&m, rt_config).unwrap();

        let vm = compiler.compile().unwrap().into_execution_context();

        let invoke0 = unsafe { vm.get_function_checked(
            m.lookup_exported_func("__app_invoke0").unwrap()
        ) };
        let invoke1 = unsafe { vm.get_function_checked(
            m.lookup_exported_func("__app_invoke1").unwrap()
        ) };
        let invoke2 = unsafe { vm.get_function_checked(
            m.lookup_exported_func("__app_invoke2").unwrap()
        ) };
        let invoke3 = unsafe { vm.get_function_checked(
            m.lookup_exported_func("__app_invoke3").unwrap()
        ) };
        let invoke4 = unsafe { vm.get_function_checked(
            m.lookup_exported_func("__app_invoke4").unwrap()
        ) };

        let app = Rc::new(ApplicationImpl {
            name: config.name.clone(),
            currently_inside: Cell::new(0),
            module: m,
            execution: vm,
            start_time: SystemTime::now(),
            invoke0_fn: invoke0,
            invoke1_fn: invoke1,
            invoke2_fn: invoke2,
            invoke3_fn: invoke3,
            invoke4_fn: invoke4,
            container: container,
            tasks: RefCell::new(Slab::new())
        });

        let mut resolver = LssaResolver::new(Rc::downgrade(&app));
        resolver.init_default_namespaces();

        app.execution.set_native_resolver(resolver);

        Application {
            inner: app
        }
    }

    pub fn initialize(&self, initializer_name: Option<&str>) {
        let _inside = AppInsideHandle::new(self);

        let initializer_name = initializer_name.unwrap_or("__app_init");

        let entry_id = match self.module.lookup_exported_func(initializer_name) {
            Some(v) => v,
            None => return
        };

        let entry: extern "C" fn () -> i64 = unsafe {
            self.execution.get_function_checked(entry_id)
        };

        let ret = entry();
        if ret != 0 {
            panic!("initialize: Initializer reported failure");
        }
    }

    pub fn add_task(&self, task: TaskInfo) -> usize {
        self.tasks.borrow_mut().insert(task)
    }

    pub fn stats(&self) -> AppStats {
        let dt: chrono::DateTime<chrono::Utc> = chrono::DateTime::from(self.start_time);
        let diff: chrono::Duration = chrono::Duration::from_std(
            SystemTime::now().duration_since(self.start_time).unwrap()
        ).unwrap();
        AppStats {
            start_time: dt.timestamp_millis(),
            running_time: diff.num_milliseconds()
        }
    }
}

impl ApplicationImpl {
    pub fn check_permission(&self, perm: &AppPermission) -> Result<(), ()> {
        let id = self.container.lookup_app_id_by_name(&self.name).unwrap();

        let cs = self.container.config_state.read().unwrap();
        let app_config = &cs.config.applications[id];

        if !app_config.metadata.permissions.contains(perm) {
            derror!(logger!(&self.name), "Permission denied: {:?}", perm);
            Err(())
        } else {
            Ok(())
        }
    }

    pub fn id(&self) -> usize {
        self.container.lookup_app_id_by_name(&self.name).unwrap()
    }

    pub fn invoke0(&self, target: i32) -> i32 {
        (self.invoke0_fn)((target as u32) as _) as _
    }

    pub fn invoke1(
        &self,
        target: i32,
        arg1: i32
    ) -> i32 {
        (self.invoke1_fn)(
            (target as u32) as _,
            (arg1 as u32) as _
        ) as _
    }

    pub fn invoke2(
        &self,
        target: i32,
        arg1: i32,
        arg2: i32
    ) -> i32 {
        (self.invoke2_fn)(
            (target as u32) as _,
            (arg1 as u32) as _,
            (arg2 as u32) as _
        ) as _
    }
}
