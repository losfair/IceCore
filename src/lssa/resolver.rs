use wasm_core::executor::{NativeResolver, NativeEntry};
use super::app::ApplicationImpl;
use std::rc::Weak;
use std::collections::BTreeMap;
use super::namespace::Namespace;

pub type NullResolver = ::wasm_core::resolver::NullResolver;

pub struct LssaResolver<I: NativeResolver> {
    module_name: String,
    prefix: String,
    app: Weak<ApplicationImpl>,
    namespaces: BTreeMap<String, Box<Namespace>>,
    next: I
}

impl<I: NativeResolver> NativeResolver for LssaResolver<I> {
    fn resolve(&self, module: &str, field: &str) -> Option<NativeEntry> {
        self.resolve_local(module, field)
            .or_else(|| self.next.resolve(module, field))
    }
}

impl<I: NativeResolver> LssaResolver<I> {
    pub fn new<S: Into<String>, T: Into<String>>(
        app: Weak<ApplicationImpl>,
        module: S,
        prefix: T,
        next: I
    ) -> LssaResolver<I> {
        LssaResolver {
            module_name: module.into(),
            prefix: prefix.into(),
            app: app,
            namespaces: BTreeMap::new(),
            next: next
        }
    }

    pub fn add_namespace<T: Namespace>(&mut self, ns: T) {
        let prefix = ns.prefix().to_string();
        self.namespaces.insert(prefix, Box::new(ns));
    }

    pub fn init_cwa_namespaces(&mut self) {
        use super::cwa;
        let app = self.app.clone();

        self.add_namespace(cwa::log::LogNs::new(
            cwa::log::LogImpl,
            app.clone()
        ));

        self.add_namespace(cwa::runtime::RuntimeNs::new(
            cwa::runtime::RuntimeImpl,
            app.clone()
        ));

        self.add_namespace(cwa::env::EnvNs::new(
            cwa::env::EnvImpl,
            app.clone()
        ));
    }

    pub fn init_ice_namespaces(&mut self) {
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

    fn resolve_local(&self, module: &str, field: &str) -> Option<NativeEntry> {
        dinfo!(logger!("resolve_local"), "Resolve: {} {}", module, field);
        if module != &self.module_name {
            return None;
        }

        match field {
            _ => {
                let full_path = match field.splitn(2, &self.prefix).nth(1) {
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
