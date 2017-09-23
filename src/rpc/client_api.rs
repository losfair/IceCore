use std;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use rpc::client::{RpcClient, RpcClientConnection};
use futures;
use futures::Future;
use executor;
use rpc::param::Param;

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_client_create(addr: *const c_char) -> *mut RpcClient {
    let addr = CStr::from_ptr(addr).to_str().unwrap();
    let client = Box::new(RpcClient::new(addr));

    Box::into_raw(client)
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_client_connect(
    client: &RpcClient,
    cb: extern "C" fn (*mut RpcClientConnection, *const c_void),
    call_with: *const c_void
) {
    let client = client.clone();
    let call_with = call_with as usize;

    executor::get_event_loop().spawn(move |_| {
        client.connect().then(move |r| -> futures::future::FutureResult<(), ()> {
            let call_with = call_with as *const c_void;
            let conn_ptr = match r {
                Ok(conn) => Box::into_raw(Box::new(conn)),
                Err(e) => {
                    eprintln!("{}", e);
                    std::ptr::null_mut()
                }
            };
            cb(conn_ptr, call_with);
            futures::future::ok(())
        })
    });
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_client_connection_destroy(conn: *mut RpcClientConnection) {
    Box::from_raw(conn);
}

#[no_mangle]
pub unsafe extern "C" fn ice_rpc_client_connection_call(
    conn: &RpcClientConnection,
    method_name: *const c_char,
    params: *const *mut Param,
    num_params: u32,
    cb: extern "C" fn (ret: *const Param, call_with: *const c_void),
    call_with: *const c_void
) {
    let params: Vec<Param> = std::slice::from_raw_parts(params, num_params as usize)
        .iter()
        .map(|v| *Box::from_raw(*v))
        .collect();
    let method_name = CStr::from_ptr(method_name).to_str().unwrap().to_string();
    let call_with = call_with as usize;

    let f = conn.call(method_name, params);

    executor::get_event_loop().spawn(move |_| f.then(move |result| {
        let call_with = call_with as *const c_void;
        match result {
            Ok(v) => cb(&v, call_with),
            Err(e) => {
                eprintln!("{}", e);
                cb(std::ptr::null(), call_with)
            }
        }
        futures::future::ok(())
    }));
}
