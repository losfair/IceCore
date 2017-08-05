use llvm_sys;
use llvm_sys::core::*;
use llvm_sys::prelude::*;

pub enum ValueType {
    Int32,
    Int64
}

impl ValueType {
    pub fn get_ref(&self) -> LLVMTypeRef {
        unsafe {
            match self {
                &ValueType::Int32 => LLVMInt32Type(),
                &ValueType::Int64 => LLVMInt64Type()
            }
        }
    }
}
