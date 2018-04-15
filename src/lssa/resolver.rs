use wasm_core::executor::{NativeResolver, NativeEntry};
use super::app::ApplicationImpl;
use std::rc::Weak;
use std::collections::BTreeMap;
use super::namespace::Namespace;

pub struct LssaResolver {
    app: Weak<ApplicationImpl>,
    namespaces: BTreeMap<String, Box<Namespace>>
}

impl NativeResolver for LssaResolver {
    fn resolve(&self, module: &str, field: &str) -> Option<NativeEntry> {
        dinfo!(logger!("resolve"), "Resolve: {} {}", module, field);
        if module != "env" {
            return None;
        }

        match field {
            _ => {
                let full_path = match field.split("__ice_").nth(1) {
                    Some(v) => v,
                    None => return None
                };
                let mut parts = full_path.splitn(2, '_');
                let ns_name = match parts.next() {
                    Some(v) => v,
                    None => return None
                };
                let field_name = match parts.next() {
                    Some(v) => v,
                    None => return None
                };
                let ns = match self.namespaces.get(ns_name) {
                    Some(v) => v,
                    None => return None
                };
                ns.dispatch(field_name)
            }
        }
    }
}

impl LssaResolver {
    pub fn new(app: Weak<ApplicationImpl>) -> LssaResolver {
        LssaResolver {
            app: app,
            namespaces: BTreeMap::new()
        }
    }

    pub fn add_namespace<T: Namespace>(&mut self, ns: T) {
        let prefix = ns.prefix().to_string();
        self.namespaces.insert(prefix, Box::new(ns));
    }

    pub fn init_default_namespaces(&mut self) {
        use super::ns;
        let app = self.app.clone();

        self.add_namespace(ns::timer::TimerNs::new(
            ns::timer::TimerImpl,
            app.clone()
        ));
        self.add_namespace(ns::logging::LoggingNs::new(
            ns::logging::LoggingImpl,
            app.clone()
        ));
        self.add_namespace(ns::tcp::TcpNs::new(
            ns::tcp::TcpImpl::new(),
            app.clone()
        ));
        self.add_namespace(ns::file::FileNs::new(
            ns::file::FileImpl::new(),
            app.clone()
        ));
    }
}
