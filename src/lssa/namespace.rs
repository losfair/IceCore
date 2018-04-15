use wasm_core::executor::{NativeEntry, GlobalStateProvider};
use wasm_core::value::Value;
use std::rc::Weak;
use super::app::ApplicationImpl;

pub trait Namespace: 'static {
    fn prefix(&self) -> &str;
    fn dispatch(&self, field: &str) -> Option<NativeEntry>;
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

macro_rules! decl_namespace {
    ($name:ident, $prefix:expr, $inner_ty:ty $(, $case:ident)*) => {
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
        }
    }
}
