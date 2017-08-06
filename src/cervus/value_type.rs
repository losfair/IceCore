use llvm_sys;
use llvm_sys::core::*;
use llvm_sys::prelude::*;

pub enum ValueType {
    Void,
    Int8,
    Int32,
    Int64,
    Float64
}

impl ValueType {
    pub fn get_ref(&self) -> LLVMTypeRef {
        unsafe {
            match self {
                &ValueType::Void => LLVMVoidType(),
                &ValueType::Int8 => LLVMInt8Type(),
                &ValueType::Int32 => LLVMInt32Type(),
                &ValueType::Int64 => LLVMInt64Type(),
                &ValueType::Float64 => LLVMFloatType()
            }
        }
    }
}
