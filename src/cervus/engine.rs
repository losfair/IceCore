use std;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic;
use llvm_sys;
use llvm_sys::target::*;
use llvm_sys::core::*;
use llvm_sys::analysis::*;
use llvm_sys::execution_engine::*;
use llvm_sys::prelude::*;
use logging;
use cervus::value_type::ValueType;

lazy_static! {
    static ref GLOBAL_INIT_DONE: atomic::AtomicBool = atomic::AtomicBool::new(false);
}

pub unsafe fn init() {
    if GLOBAL_INIT_DONE.fetch_or(true, atomic::Ordering::SeqCst) {
        return;
    }

    let logger = logging::Logger::new("cervus::engine::init");
    logger.log(logging::Message::Info("Initializing LLVM".to_string()));
    LLVMLinkInMCJIT();
    LLVM_InitializeNativeTarget();
    logger.log(logging::Message::Info("Done".to_string()));
}

pub struct Module {
    _ref: LLVMModuleRef
}

impl Module {
    pub fn new(name: &str) -> Module {
        unsafe {
            init();
        }

        let name = CString::new(name).unwrap();
        let mod_ref = unsafe { LLVMModuleCreateWithName(name.as_ptr()) };
        Module {
            _ref: mod_ref
        }
    }
}

pub struct ExecutionEngine<'a> {
    module: &'a Module,
    _ref: LLVMExecutionEngineRef
}

impl<'a> ExecutionEngine<'a> {
    pub fn new(module: &'a Module) -> ExecutionEngine<'a> {
        unsafe {
            let mut err_str: *mut c_char = std::ptr::null_mut();
            LLVMVerifyModule(module._ref, LLVMVerifierFailureAction::LLVMAbortProcessAction, &mut err_str as *mut *mut c_char);
            LLVMDisposeMessage(err_str);

            let mut ee: LLVMExecutionEngineRef = std::ptr::null_mut();
            let ret = LLVMCreateExecutionEngineForModule(&mut ee as *mut LLVMExecutionEngineRef, module._ref, &mut err_str as *mut *mut c_char);

            if ret != 0 {
                panic!("Unable to create execution engine");
            }

            if !err_str.is_null() {
                panic!("{}", CStr::from_ptr(err_str).to_str().unwrap());
                //LLVMDisposeMessage(err_str);
            }

            ExecutionEngine {
                module: module,
                _ref: ee
            }
        }
    }

    pub unsafe fn run(&self, f: &Function, args: Vec<GenericValue>) -> GenericValue {
        let mut args: Vec<LLVMGenericValueRef> = args.iter().map(|v| v._ref).collect();
        GenericValue {
            _ref: LLVMRunFunction(self._ref, f._ref, args.len() as u32, args.as_mut_ptr() as *mut LLVMGenericValueRef)
        }
    }
}

impl<'a> Drop for ExecutionEngine<'a> {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeExecutionEngine(self._ref);
        }
    }
}

pub struct Function<'a> {
    module: &'a Module,
    name: String,
    ret_type: ValueType,
    param_types: Vec<ValueType>,
    _ref: LLVMValueRef
}

impl<'a> Function<'a> {
    pub fn new(module: &'a Module, name: &str, ret_type: ValueType, param_types: Vec<ValueType>) -> Function<'a> {
        let fn_ref = unsafe {
            let mut raw_pt = Vec::with_capacity(param_types.len());
            for t in &param_types {
                raw_pt.push(t.get_ref());
            }
            let ret_type_ref = ret_type.get_ref();
            let fn_type = LLVMFunctionType(ret_type_ref, raw_pt.as_mut_ptr(), raw_pt.len() as u32, 0);
            LLVMAddFunction(module._ref, CString::new(name).unwrap().as_ptr(), fn_type)
        };

        Function {
            module: module,
            name: name.to_owned(),
            ret_type: ret_type,
            param_types: param_types,
            _ref: fn_ref
        }
    }

    pub fn get_param(&self, index: usize) -> Option<Value> {
        if index < self.param_types.len() {
            Some(Value {
                _ref: unsafe {
                    LLVMGetParam(self._ref, index as u32)
                }
            })
        } else {
            None
        }
    }
}

pub struct BasicBlock<'a> {
    func: &'a Function<'a>,
    name: String,
    _ref: LLVMBasicBlockRef
}

impl<'a> BasicBlock<'a> {
    pub fn new(func: &'a Function, name: &str) -> BasicBlock<'a> {
        let bb_ref = unsafe {
            LLVMAppendBasicBlock(func._ref, CString::new(name).unwrap().as_ptr())
        };

        BasicBlock {
            func: func,
            name: name.to_owned(),
            _ref: bb_ref
        }
    }
}

pub struct Builder<'a> {
    basic_block: &'a BasicBlock<'a>,
    next_action_id: u32,
    _ref: LLVMBuilderRef
}

impl<'a> Builder<'a> {
    pub fn new(bb: &'a BasicBlock) -> Builder<'a> {
        let builder_ref = unsafe {
            let r = LLVMCreateBuilder();
            LLVMPositionBuilderAtEnd(r, bb._ref);
            r
        };
        Builder {
            basic_block: bb,
            next_action_id: 0,
            _ref: builder_ref
        }
    }

    pub fn append(&mut self, act: Action) {
        act.build(self);
    }
}

impl<'a> Drop for Builder<'a> {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeBuilder(self._ref);
        }
    }
}

pub struct Value {
    _ref: LLVMValueRef
}

pub struct GenericValue {
    _ref: LLVMGenericValueRef
}

impl GenericValue {
}

impl From<i32> for GenericValue {
    fn from(s: i32) -> GenericValue {
        unsafe {
            GenericValue {
                _ref: LLVMCreateGenericValueOfInt(LLVMInt32Type(), s as u64, 1)
            }
        }
    }
}

impl From<i64> for GenericValue {
    fn from(s: i64) -> GenericValue {
        unsafe {
            GenericValue {
                _ref: LLVMCreateGenericValueOfInt(LLVMInt64Type(), s as u64, 1)
            }
        }
    }
}

impl From<u32> for GenericValue {
    fn from(s: u32) -> GenericValue {
        unsafe {
            GenericValue {
                _ref: LLVMCreateGenericValueOfInt(LLVMInt32Type(), s as u64, 0)
            }
        }
    }
}

impl From<u64> for GenericValue {
    fn from(s: u64) -> GenericValue {
        unsafe {
            GenericValue {
                _ref: LLVMCreateGenericValueOfInt(LLVMInt64Type(), s as u64, 0)
            }
        }
    }
}

impl From<f32> for GenericValue {
    fn from(s: f32) -> GenericValue {
        unsafe {
            GenericValue {
                _ref: LLVMCreateGenericValueOfFloat(LLVMFloatType(), s as f64)
            }
        }
    }
}

impl From<f64> for GenericValue {
    fn from(s: f64) -> GenericValue {
        unsafe {
            GenericValue {
                _ref: LLVMCreateGenericValueOfFloat(LLVMFloatType(), s as f64)
            }
        }
    }
}

pub enum Action {
    Add(Value, Value),
    Return(Value)
}

impl Action {
    fn build(&self, builder: &mut Builder) -> LLVMValueRef {
        builder.next_action_id += 1;
        let action_name = CString::new(format!("action_{}", builder.next_action_id)).unwrap();

        unsafe {
            match self {
                &Action::Add(ref left, ref right) => LLVMBuildAdd(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::Return(ref v) => LLVMBuildRet(builder._ref, v._ref)
            }
        }
    }
}
