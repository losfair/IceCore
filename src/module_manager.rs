use std;
use std::rc::Rc;
use std::cell::RefCell;
use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::collections::HashMap;
use std::sync::{Arc, Weak, Mutex, RwLock};
use std::ffi::CStr;
use std::os::raw::{c_void, c_char};
use cervus;
use cervus::value_type::ValueType;
use logging;
use delegates;
use glue;

pub struct Modules {
    mods: HashMap<String, ModuleRuntime>,
    all_hooks: Arc<Mutex<HashMap<String, Vec<Weak<ModuleContext>>>>>
}

pub struct ModuleRuntime {
    ee: cervus::engine::ExecutionEngine,
    context: Arc<ModuleContext>
}

pub struct ModuleContext {
    all_hooks: Arc<Mutex<HashMap<String, Vec<Weak<ModuleContext>>>>>,
    hooks: Mutex<HashMap<String, extern fn (*const HookContext)>>,
    context_mem: RwLock<Option<Vec<u8>>>
}

impl ModuleRuntime {
    fn with_context(ee: cervus::engine::ExecutionEngine, ctx: Arc<ModuleContext>) -> ModuleRuntime {
        ModuleRuntime {
            ee: ee,
            context: ctx
        }
    }
}

impl ModuleContext {
    fn from_global(m: &Modules) -> ModuleContext {
        ModuleContext {
            all_hooks: m.all_hooks.clone(),
            hooks: Mutex::new(HashMap::new()),
            context_mem: RwLock::new(None)
        }
    }
}

pub struct HookContext {
    inner: Box<Any>
}

impl Deref for HookContext {
    type Target = Box<Any>;
    fn deref(&self) -> &Box<Any> {
        &self.inner
    }
}

impl DerefMut for HookContext {
    fn deref_mut(&mut self) -> &mut Box<Any> {
        &mut self.inner
    }
}

impl From<Box<Any>> for HookContext {
    fn from(v: Box<Any>) -> HookContext {
        HookContext {
            inner: v
        }
    }
}

impl Into<Box<Any>> for HookContext {
    fn into(self) -> Box<Any> {
        self.inner
    }
}

impl Modules {
    pub fn new() -> Modules {
        Modules {
            mods: HashMap::new(),
            all_hooks: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn load(&mut self, name: &str, bitcode: &[u8]) {
        let mod_logger = logging::Logger::new(name);
        let ctx = Arc::new(ModuleContext::from_global(self));

        let m = cervus::engine::Module::from_bitcode(name, bitcode).unwrap();
        let m = cervus::logging::set_logger(m, Box::new(move |level, msg| {
            print_module_log(
                &mod_logger,
                level,
                msg
            );
        }));
        let m = cervus::patcher::add_local_fn(
            m,
            "add_hook",
            _add_hook as *const c_void,
            ValueType::Void,
            vec![
                cervus::patcher::Argument::Local(
                    Box::new(Arc::downgrade(&ctx))
                ),
                cervus::patcher::Argument::FromCall(
                    ValueType::Pointer(
                        Box::new(ValueType::Int8)
                    )
                ),
                cervus::patcher::Argument::FromCall(
                    ValueType::Pointer(
                        Box::new(ValueType::Void)
                    )
                )
            ]
        );
        let m = cervus::patcher::add_local_fn(
            m,
            "reset_context_mem",
            _reset_context_mem as *const c_void,
            ValueType::Pointer(
                Box::new(ValueType::Void)
            ),
            vec![
                cervus::patcher::Argument::Local(
                    Box::new(Arc::downgrade(&ctx))
                ),
                cervus::patcher::Argument::FromCall(
                    ValueType::Int32
                )
            ]
        );
        let m = cervus::patcher::add_local_fn(
            m,
            "get_context_mem",
            _get_context_mem as *const c_void,
            ValueType::Pointer(
                Box::new(ValueType::Void)
            ),
            vec![
                cervus::patcher::Argument::Local(
                    Box::new(Arc::downgrade(&ctx))
                )
            ]
        );
        let m = cervus::patcher::add_local_fn(
            m,
            "downcast_hook_context",
            _downcast_hook_context as *const c_void,
            ValueType::Pointer(
                Box::new(ValueType::Void)
            ),
            vec![
                cervus::patcher::Argument::FromCall(
                    ValueType::Pointer(Box::new(ValueType::Void))
                ),
                cervus::patcher::Argument::FromCall(
                    ValueType::Pointer(Box::new(ValueType::Int8))
                )
            ]
        );

        let ee = cervus::engine::ExecutionEngine::new(m);
        let initializer = ee.get_callable_0::<()>(
            &cervus::engine::Function::new_null_handle(
                "module_init",
                ValueType::Void,
                vec![]
            )
        );

        initializer();

        self.mods.insert(name.to_string(), ModuleRuntime::with_context(
            ee,
            ctx
        ));
    }

    pub fn run_hooks_by_name<T>(&self, name: &str, hook_ctx: Box<T>) -> Box<T> where T: 'static, Box<T>: Into<Box<Any>> {
        let all_hooks = self.all_hooks.lock().unwrap();
        let hook_ctx = hook_ctx.into();
        let hook_ctx = HookContext::from(hook_ctx);

        match all_hooks.get(name) {
            Some(t) => {
                for mc in t {
                    let mc = match mc.upgrade() {
                        Some(v) => v,
                        None => {
                            continue;
                        }
                    };
                    let hooks = mc.hooks.lock().unwrap();
                    match hooks.get(name) {
                        Some(f) => {
                            f(&hook_ctx);
                        },
                        None => {
                            continue;
                        }
                    }
                }
            },
            None => {}
        }

        let hook_ctx: Box<Any> = hook_ctx.into();
        hook_ctx.downcast::<T>().unwrap()
    }
}

fn print_module_log(logger: &logging::Logger, level: cervus::logging::LogLevel, msg: &str) {
    let msg = msg.to_string();

    use cervus::logging::LogLevel;
    logger.log(
        match level {
            LogLevel::Emergency | LogLevel::Alert | LogLevel::Critical | LogLevel::Error => logging::Message::Error(msg),
            LogLevel::Warning | LogLevel::Notice => logging::Message::Warning(msg),
            LogLevel::Info | LogLevel::Debug => logging::Message::Info(msg)
        }
    );
}

unsafe extern fn _add_hook(
    m: *const cervus::engine::ModuleResource,
    name: *const c_char,
    cb: extern fn (*const HookContext)
) {
    let m = (&*m).downcast_ref::<Weak<ModuleContext>>().unwrap().upgrade().unwrap();
    let name = CStr::from_ptr(name).to_str().unwrap();

    m.hooks.lock().unwrap().insert(
        name.to_owned(),
        cb
    );

    let mut all_hooks = m.all_hooks.lock().unwrap();
    let entry = all_hooks.entry(name.to_owned()).or_insert(Vec::new());

    entry.push(Arc::downgrade(&m));
}

unsafe extern fn _reset_context_mem(
    m: *const cervus::engine::ModuleResource,
    size: u32
) -> *mut u8 {
    let m = (&*m).downcast_ref::<Weak<ModuleContext>>().unwrap().upgrade().unwrap();
    if size == 0 {
        *m.context_mem.write().unwrap() = None;
        std::ptr::null_mut()
    } else {
        let mut v = vec![0; size as usize];
        let addr = v.as_mut_ptr();
        *m.context_mem.write().unwrap() = Some(v);
        addr
    }
}

unsafe extern fn _get_context_mem(
    m: *const cervus::engine::ModuleResource
) -> *mut u8 {
    let m = (&*m).downcast_ref::<Weak<ModuleContext>>().unwrap().upgrade().unwrap();
    let ret = match *m.context_mem.read().unwrap() {
        Some(ref v) => v.as_ptr() as *mut u8,
        None => std::ptr::null_mut()
    };
    ret
}

unsafe extern fn _downcast_hook_context(
    hc: *const HookContext,
    target_type: *const c_char
) -> *const c_void {
    let hc = &*hc;
    let target_type = CStr::from_ptr(target_type).to_str().unwrap();

    match target_type {
        "basic_request_info" => {
            hc.downcast_ref::<delegates::BasicRequestInfo>().unwrap()
                as *const delegates::BasicRequestInfo
                as *const c_void
        },
        "glue_response" => {
            hc.downcast_ref::<glue::response::Response>().unwrap()
                as *const glue::response::Response
                as *const c_void
        },
        _ => panic!("Downcast failed: Unknown target type")
    }
}
