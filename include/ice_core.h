typedef void * Resource;
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;

typedef void (*AsyncEndpointHandler) (int id, Resource call_info);
typedef Resource (*CallbackOnRequest) (const char *uri); // returns a Response

#ifdef __cplusplus
extern "C" {
#endif

Resource ice_create_server();
Resource ice_server_listen(Resource handle, const char *addr);
Resource ice_server_router_add_endpoint(Resource handle, const char *p);

const char * ice_glue_request_get_remote_addr(Resource req);
const char * ice_glue_request_get_method(Resource req);
const char * ice_glue_request_get_uri(Resource req);

const char * ice_glue_add_header(Resource t, const char *k, const char *v);
const char * ice_glue_get_header(Resource t, const char *k);

Resource ice_glue_create_response();
void ice_glue_response_set_body(Resource t, const u8 *body, u32 len);
const char * ice_glue_request_get_body(Resource t, u32 *len_out);

void ice_glue_register_async_endpoint_handler(AsyncEndpointHandler);

void ice_core_fire_callback(Resource call_info, Resource resp);
Resource ice_core_borrow_request_from_call_info(Resource call_info);
int ice_core_endpoint_get_id(Resource ep);

void ice_core_endpoint_set_flag(Resource ep, const char *name, bool value);

#ifdef __cplusplus
}
#endif
