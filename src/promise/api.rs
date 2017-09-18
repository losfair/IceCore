use std;
use std::os::raw::c_char;
use std::ffi::CStr;
use promise::chain;
use trait_handle::TraitHandle;

#[no_mangle]
pub extern "C" fn ice_promise_chain_create() -> *mut chain::Chain {
    Box::into_raw(Box::new(chain::Chain::new()))
}

#[no_mangle]
pub unsafe extern "C" fn ice_promise_chain_destroy(c: *mut chain::Chain) {
    Box::from_raw(c);
}

#[no_mangle]
pub unsafe extern "C" fn ice_promise_chain_add_step(
    c: &mut chain::Chain,
    step: *mut chain::Step
) {
    let step = Box::from_raw(step);
    c.steps.push(*step);
}

#[no_mangle]
pub unsafe extern "C" fn ice_promise_chain_into_executor(
    c: *mut chain::Chain,
    executor_name: *const c_char
) -> *mut TraitHandle<chain::ChainExecutor> {
    let c = Box::from_raw(c);
    let executor_name = CStr::from_ptr(executor_name).to_str().unwrap();

    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn ice_promise_step_create(
    target: chain::StepTarget,
    call_context: usize
) -> *mut chain::Step {
    Box::into_raw(Box::new(chain::Step {
        target: target,
        call_context: call_context
    }))
}

#[no_mangle]
pub unsafe extern "C" fn ice_promise_step_destroy(
    step: *mut chain::Step
) {
    Box::from_raw(step);
}
