#ifndef _ICE_GLUE_IMPORTS_H_
#define _ICE_GLUE_IMPORTS_H_

typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;

typedef void * Context;
typedef void * Session;

extern "C" void ice_core_destroy_cstring(char *v);

extern "C" void ice_core_destroy_context_handle(Context ctx);

extern "C" Session ice_context_create_session(Context ctx);
extern "C" Session ice_context_get_session_by_id(Context ctx, const char *id);
extern "C" char * ice_core_session_get_id(Session sess);
extern "C" void ice_core_destroy_session_handle(Session sess);
extern "C" char * ice_core_session_get_item(Session sess, const char *k);
extern "C" void ice_core_session_set_item(Session sess, const char *k, const char *v);
extern "C" void ice_core_session_remove_item(Session sess, const char *k);

extern "C" char * ice_context_render_template(Context ctx, const char *name, const char *data);

#endif
