use llvm_sys;
use llvm_sys::core::*;
use llvm_sys::prelude::*;

pub enum ValueType {
    Void,
    Int8,
    Int32,
    Int64,
    Float64,
    Pointer(Box<ValueType>),
    Function(Box<ValueType>, Vec<ValueType>)
}

impl ValueType {
    pub fn get_ref(&self) -> LLVMTypeRef {
        unsafe {
            match self {
                &ValueType::Void => LLVMVoidType(),
                &ValueType::Int8 => LLVMInt8Type(),
                &ValueType::Int32 => LLVMInt32Type(),
                &ValueType::Int64 => LLVMInt64Type(),
                &ValueType::Float64 => LLVMFloatType(),
                &ValueType::Pointer(ref inner) => LLVMPointerType(inner.get_ref(), 0),
                &ValueType::Function(ref ret_type, ref param_types) => {
                    let mut param_types_ref: Vec<LLVMTypeRef> = param_types.iter().map(|v| v.get_ref()).collect();
                    LLVMFunctionType(ret_type.get_ref(), param_types_ref.as_mut_ptr(), param_types_ref.len() as u32, 0)
                }
            }
        }
    }
}
