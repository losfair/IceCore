use wasm_core::executor::{NativeEntry, GlobalStateProvider};
use wasm_core::value::Value;
use std::rc::Weak;
use super::app::ApplicationImpl;
use serde::{Serialize, Deserialize};
use bincode;

pub trait Namespace: 'static {
    fn prefix(&self) -> &str;
    fn dispatch(&self, field: &str) -> Option<NativeEntry>;
    fn start_migration(&self) -> Option<Migration>;
    fn complete_migration(&self, migration: &Migration);
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone)]
pub struct Migration {
    state: Vec<u8> // serialized
}

pub trait MigrationProvider<T: Namespace> {
    fn start_migration(target: &T) -> Option<Migration>;
    fn complete_migration(target: &T, migration: &Migration);
}

pub struct NullMigrationProvider;
impl<T: Namespace> MigrationProvider<T> for NullMigrationProvider {
    fn start_migration(_: &T) -> Option<Migration> {
        Some(Migration::empty())
    }
    fn complete_migration(_: &T, _migration: &Migration) {}
}

#[allow(dead_code)]
impl Migration {
    pub fn empty() -> Migration {
        Migration {
            state: vec! []
        }
    }

    pub fn new<T: Serialize>(v: &T) -> Migration {
        Migration {
            state: bincode::serialize(v).unwrap()
        }
    }

    pub fn extract<'a, T: Deserialize<'a>>(&'a self) -> Option<T> {
        match bincode::deserialize(&self.state) {
            Ok(v) => Some(v),
            Err(e) => {
                derror!(
                    logger!("Migration::extract"),
                    "Unable to extract state: {:?}",
                    e
                );
                None
            }
        }
    }
}

pub struct InvokeContext<'a> {
    pub state: &'a mut GlobalStateProvider,
    pub args: &'a [Value],
    pub app: &'a Weak<ApplicationImpl>
}

#[allow(dead_code)]
impl<'a> InvokeContext<'a> {
    pub fn extract_bytes(&self, ptr_arg_index: usize, len_arg_index: usize) -> &[u8] {
        let base = self.args[ptr_arg_index].get_i32().unwrap() as usize;
        let len = self.args[len_arg_index].get_i32().unwrap() as usize;
        &self.state.get_memory()[base .. base + len]
    }

    pub fn extract_bytes_mut(&mut self, ptr_arg_index: usize, len_arg_index: usize) -> &mut [u8] {
        let base = self.args[ptr_arg_index].get_i32().unwrap() as usize;
        let len = self.args[len_arg_index].get_i32().unwrap() as usize;
        &mut self.state.get_memory_mut()[base .. base + len]
    }

    pub fn extract_str(&self, ptr_arg_index: usize, len_arg_index: usize) -> &str {
        ::std::str::from_utf8(
            self.extract_bytes(ptr_arg_index, len_arg_index)
        ).unwrap()
    }
}

macro_rules! decl_namespace_with_migration_provider {
    ($name:ident, $prefix:expr, $inner_ty:ty, $mig:tt $(, $case:ident)*) => {
        #[derive(Clone)]
        pub struct $name {
            provider: ::std::rc::Rc<$inner_ty>,
            app: ::std::rc::Weak<$crate::lssa::app::ApplicationImpl>
        }

        #[allow(dead_code)]
        impl $name {
            pub fn new(inner: $inner_ty, app: ::std::rc::Weak<$crate::lssa::app::ApplicationImpl>) -> Self {
                $name {
                    provider: ::std::rc::Rc::new(inner),
                    app: app
                }
            }

            pub fn from_rc(inner: ::std::rc::Rc<$inner_ty>, app: ::std::rc::Weak<$crate::lssa::app::ApplicationImpl>) -> Self {
                $name {
                    provider: inner,
                    app: app
                }
            }
        }

        impl $crate::lssa::namespace::Namespace for $name {
            fn prefix(&self) -> &str {
                $prefix
            }

            fn dispatch(&self, field: &str) -> Option<::wasm_core::executor::NativeEntry> {
                let provider = self.provider.clone();
                let app = self.app.clone();

                match field {
                    $(
                        stringify!($case) => Some(Box::new(move |state, args| {
                            let ctx = $crate::lssa::namespace::InvokeContext {
                                state: state,
                                args: args,
                                app: &app
                            };
                            Ok(provider.$case(ctx))
                        })),
                    )*
                    _ => None
                }
            }

            fn start_migration(&self) -> Option<$crate::lssa::namespace::Migration> {
                use $crate::lssa::namespace::MigrationProvider;
                $mig::start_migration(self)
            }

            fn complete_migration(&self, mig: &$crate::lssa::namespace::Migration) {
                use $crate::lssa::namespace::MigrationProvider;
                $mig::complete_migration(self, mig)
            }
        }
    }
}

macro_rules! decl_namespace {
    ($name:ident, $prefix:expr, $inner_ty:ty $(, $case:ident)*) => {
        use $crate::lssa::namespace::NullMigrationProvider;
        decl_namespace_with_migration_provider!(
            $name,
            $prefix,
            $inner_ty,
            NullMigrationProvider
            $(, $case)*
        );
    }
}
