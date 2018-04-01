use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::ops::Deref;

use wasm_core;
use wasm_core::jit::compiler::{Compiler, ExecutionContext};
use wasm_core::jit::runtime::RuntimeConfig;
use wasm_core::module::{Module, Type};
use container::Container;

use super::task::TaskInfo;
use super::resolver::LssaResolver;
use slab::Slab;

pub struct Migration {
    app: Application
}

unsafe impl Send for Migration {}

impl Migration {
    pub fn unwrap(self) -> Application {
        self.app
    }
}

// `inner` is intended to be used internally only and this should NOT be `Clone`.
pub struct Application {
    inner: Rc<ApplicationImpl>
}

pub struct ApplicationImpl {
    pub(super) name: String,
    currently_inside: Cell<usize>,
    module: Module,
    execution: ExecutionContext,
    inner_task_dispatcher_fn: usize,
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

        let inner_task_dispatcher_fn = Self::find_inner_dispatcher(&m);

        let app = Rc::new(ApplicationImpl {
            name: config.name.clone(),
            currently_inside: Cell::new(0),
            module: m,
            execution: vm,
            inner_task_dispatcher_fn: inner_task_dispatcher_fn,
            container: container,
            tasks: RefCell::new(Slab::new())
        });

        let resolver = LssaResolver::new(Rc::downgrade(&app));
        app.execution.set_native_resolver(resolver);

        Application {
            inner: app
        }
    }

    fn find_inner_dispatcher(m: &Module) -> usize {
        let entry_id = m.lookup_exported_func("app_task_dispatch").unwrap_or_else(|| panic!("app_task_dispatch not found"));
        let typeidx = m.functions[entry_id].typeidx as usize;
        let Type::Func(ref ty_args, ref ty_ret) = m.types[typeidx];

        if ty_args.len() != 1 {
            panic!("find_inner_dispatcher: Expected exactly one argument");
        }

        if ty_ret.len() != 1 {
            panic!("find_inner_dispatcher: Expected exactly one return value");
        }

        entry_id
    }

    pub fn initialize(&self, initializer_name: Option<&str>) {
        let _inside = AppInsideHandle::new(self);

        let initializer_name = initializer_name.unwrap_or("app_init");

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

    pub fn into_migration(self) -> Result<Migration, Self> {
        if self.currently_inside.get() != 0 {
            return Err(self);
        }

        Ok(Migration {
            app: self
        })
    }

    pub fn add_task(&self, task: TaskInfo) -> usize {
        self.tasks.borrow_mut().insert(task)
    }

    pub fn invoke_inner_dispatcher_on_task(&self, task_id: usize) {
        let f: extern "C" fn (task_id: i64) -> i64 = unsafe { self.execution.get_function_checked(
            self.inner_task_dispatcher_fn
        ) };

        let ret = f(task_id as i64);
        if ret != 0 {
            panic!("invoke_inner_dispatcher_on_task: Inner dispatcher reported failure");
        }
    }
}
