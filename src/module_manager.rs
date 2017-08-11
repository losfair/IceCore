use std::collections::HashMap;
use std::sync::{Arc, Weak, Mutex};
use std::ffi::CStr;
use std::os::raw::{c_void, c_char};
use cervus;
use cervus::value_type::ValueType;
use logging;

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
    hooks: Mutex<HashMap<String, extern fn (*const HookContext)>>
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
            hooks: Mutex::new(HashMap::new())
        }
    }
}

pub struct HookContext {
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

    pub fn run_hooks_by_name(&self, name: &str, hook_ctx: &HookContext) {
        let all_hooks = self.all_hooks.lock().unwrap();
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
                            f(hook_ctx);
                        },
                        None => {
                            continue;
                        }
                    }
                }
            },
            None => {}
        }
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
