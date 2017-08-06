use std;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::atomic;
use llvm_sys;
use llvm_sys::target::*;
use llvm_sys::target_machine::*;
use llvm_sys::core::*;
use llvm_sys::transforms::scalar::*;
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
    LLVM_InitializeNativeAsmPrinter();
    LLVM_InitializeNativeAsmParser();
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
    _ref: LLVMExecutionEngineRef,
    _pm_ref: LLVMPassManagerRef
}

impl<'a> ExecutionEngine<'a> {
    pub fn new(module: &'a Module) -> ExecutionEngine<'a> {
        unsafe {
            let mut err_str: *mut c_char = std::ptr::null_mut();
            LLVMVerifyModule(module._ref, LLVMVerifierFailureAction::LLVMAbortProcessAction, &mut err_str);
            LLVMDisposeMessage(err_str);
            err_str = std::ptr::null_mut();

            let mut ee: LLVMExecutionEngineRef = std::ptr::null_mut();
            let mut mcjit_options = LLVMMCJITCompilerOptions {
                OptLevel: 3,
                CodeModel: LLVMCodeModel::LLVMCodeModelJITDefault,
                NoFramePointerElim: 0,
                EnableFastISel: 0,
                MCJMM: std::ptr::null_mut()
            };

            LLVMInitializeMCJITCompilerOptions(&mut mcjit_options, std::mem::size_of::<LLVMMCJITCompilerOptions>());
            //mcjit_options.OptLevel = 3;
            let ret = LLVMCreateMCJITCompilerForModule(&mut ee, module._ref, &mut mcjit_options, std::mem::size_of::<LLVMMCJITCompilerOptions>(), &mut err_str);

            if ret != 0 {
                panic!("Unable to create execution engine");
            }

            if !err_str.is_null() {
                LLVMDisposeMessage(err_str);
                err_str = std::ptr::null_mut();
            }

            let pm = LLVMCreatePassManager();
            LLVMAddConstantPropagationPass(pm);
            LLVMAddInstructionCombiningPass(pm);
            LLVMAddGVNPass(pm);

            ExecutionEngine {
                module: module,
                _ref: ee,
                _pm_ref: pm
            }
        }
    }

    pub fn prepare(&self) {
        unsafe {
            LLVMRunPassManager(self._pm_ref, self.module._ref);
        }
    }

    pub fn get_raw_callable(&self, f: &Function) -> *const c_void {
        unsafe {
            let fn_name = f.name.as_str();

            let f = LLVMGetFunctionAddress(self._ref, CString::new(fn_name).unwrap().as_ptr()) as usize;
            if f == 0 {
                panic!("Unable to get function address for: {}", fn_name);
            }

            f as *const c_void
        }
    }

    pub fn get_callable_0<R>(&self, f: &Function) -> extern fn () -> R {
        unsafe {
            std::mem::transmute::<*const c_void, extern fn () -> R>(self.get_raw_callable(f))
        }
    }

    pub fn get_callable_1<R, A>(&self, f: &Function) -> extern fn (A) -> R {
        unsafe {
            std::mem::transmute::<*const c_void, extern fn (A) -> R>(self.get_raw_callable(f))
        }
    }

    pub fn get_callable_2<R, A, B>(&self, f: &Function) -> extern fn (A, B) -> R {
        unsafe {
            std::mem::transmute::<*const c_void, extern fn (A, B) -> R>(self.get_raw_callable(f))
        }
    }

    pub fn get_callable_3<R, A, B, C>(&self, f: &Function) -> extern fn (A, B, C) -> R {
        unsafe {
            std::mem::transmute::<*const c_void, extern fn (A, B, C) -> R>(self.get_raw_callable(f))
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

    pub fn append(&mut self, act: Action) -> Value {
        Value {
            _ref: act.build(self)
        }
    }
}

impl<'a> Drop for Builder<'a> {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeBuilder(self._ref);
        }
    }
}

#[derive(Clone)]
pub struct Value {
    _ref: LLVMValueRef
}

impl From<i8> for Value {
    fn from(v: i8) -> Value {
        unsafe {
            Value {
                _ref: LLVMConstInt(ValueType::Int8.get_ref(), v as u64, 1)
            }
        }
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Value {
        unsafe {
            Value {
                _ref: LLVMConstInt(ValueType::Int64.get_ref(), v as u64, 1)
            }
        }
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Value {
        unsafe {
            Value {
                _ref: LLVMConstReal(ValueType::Float64.get_ref(), v)
            }
        }
    }
}

#[allow(dead_code)]
pub enum Action {
    IntAdd(Value, Value),
    FloatAdd(Value, Value),
    IntSub(Value, Value),
    FloatSub(Value, Value),
    IntMul(Value, Value),
    FloatMul(Value, Value),
    SignedIntDiv(Value, Value),
    UnsignedIntDiv(Value, Value),
    FloatDiv(Value, Value),
    And(Value, Value),
    Or(Value, Value),
    Xor(Value, Value),
    Not(Value),
    Shl(Value, Value),
    LogicalShr(Value, Value),
    ArithmeticShr(Value, Value),
    Return(Value)
}

impl Action {
    fn build(&self, builder: &mut Builder) -> LLVMValueRef {
        builder.next_action_id += 1;
        let action_name = CString::new(format!("action_{}", builder.next_action_id)).unwrap();

        unsafe {
            match self {
                &Action::IntAdd(ref left, ref right) => LLVMBuildAdd(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::FloatAdd(ref left, ref right) => LLVMBuildFAdd(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::IntSub(ref left, ref right) => LLVMBuildSub(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::FloatSub(ref left, ref right) => LLVMBuildFSub(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::IntMul(ref left, ref right) => LLVMBuildMul(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::FloatMul(ref left, ref right) => LLVMBuildFMul(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::SignedIntDiv(ref left, ref right) => LLVMBuildSDiv(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::UnsignedIntDiv(ref left, ref right) => LLVMBuildUDiv(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::FloatDiv(ref left, ref right) => LLVMBuildFDiv(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::And(ref left, ref right) => LLVMBuildAnd(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::Or(ref left, ref right) => LLVMBuildOr(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::Xor(ref left, ref right) => LLVMBuildXor(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::Not(ref v) => LLVMBuildNot(builder._ref, v._ref, action_name.as_ptr()),
                &Action::Shl(ref left, ref right) => LLVMBuildShl(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::LogicalShr(ref left, ref right) => LLVMBuildLShr(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::ArithmeticShr(ref left, ref right) => LLVMBuildAShr(builder._ref, left._ref, right._ref, action_name.as_ptr()),
                &Action::Return(ref v) => LLVMBuildRet(builder._ref, v._ref)
            }
        }
    }
}
