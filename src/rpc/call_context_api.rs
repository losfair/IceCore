use rpc::call_context::CallContext;
use rpc::param::Param;

#[no_mangle]
pub extern "C" fn ice_rpc_call_context_get_num_params(ctx: &CallContext) -> u32 {
    ctx.params.len() as u32
}

#[no_mangle]
pub extern "C" fn ice_rpc_call_context_get_param(ctx: &CallContext, pos: u32) -> *const Param {
    &ctx.params[pos as usize]
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_call_context_end(ctx: *mut CallContext, ret: *mut Param) {
    let ctx = Box::from_raw(ctx);
    ctx.end(*Box::from_raw(ret));
}
