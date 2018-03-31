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
        use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

        let mut rt_config = RuntimeConfig::default();

        rt_config.mem_default = config.mem_default;
        rt_config.mem_max = config.mem_max;

        let _inner_unsafe: Rc<ApplicationImpl> = Rc::new(unsafe {
            ::std::mem::uninitialized()
        });

        let maybe_app_impl = catch_unwind(AssertUnwindSafe(|| {
            let resolver = LssaResolver::new(Rc::downgrade(&_inner_unsafe));

            let compiler = Compiler::with_runtime_config(&m, rt_config).unwrap();
            compiler.set_native_resolver(resolver);

            let vm = compiler.compile().unwrap().into_execution_context();

            let inner_task_dispatcher_fn = Self::find_inner_dispatcher(&m);

            ApplicationImpl {
                name: config.name.clone(),
                currently_inside: Cell::new(0),
                module: m,
                execution: vm,
                inner_task_dispatcher_fn: inner_task_dispatcher_fn,
                container: container,
                tasks: RefCell::new(Slab::new())
            }
        }));

        unsafe {
            match maybe_app_impl {
                Ok(v) => {
                    // FIXME: Is casting const pointer to mut valid here ?
                    let _inner_unsafe = Rc::into_raw(_inner_unsafe);
                    ::std::ptr::write(
                        _inner_unsafe as *mut ApplicationImpl
                        , v
                    );
                    Application {
                        inner: Rc::from_raw(_inner_unsafe)
                    }
                },
                Err(e) => {
                    ::std::mem::forget(Rc::try_unwrap(_inner_unsafe)
                        .unwrap_or_else(|_| {
                            ::std::process::abort();
                        }));
                    resume_unwind(e);
                }
            }
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

        let typeidx = self.module.functions[entry_id].typeidx as usize;
        let Type::Func(ref ty_args, ref ty_ret) = self.module.types[typeidx];

        if ty_args.len() != 0 {
            panic!("initialize: Expected no arguments");
        }

        if ty_ret.len() != 1 {
            panic!("initialize: Expected exactly one return value");
        }

        let entry = self.execution.get_function_address(entry_id);
        let entry: extern "C" fn () -> i64 = unsafe {
            ::std::mem::transmute(entry)
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
        let entry = self.execution.get_function_address(
            self.inner_task_dispatcher_fn
        );

        let f: extern "C" fn (task_id: i64) -> i64 = unsafe {
            ::std::mem::transmute(entry)
        };

        let ret = f(task_id as i64);
        if ret != 0 {
            panic!("invoke_inner_dispatcher_on_task: Inner dispatcher reported failure");
        }
    }
}
