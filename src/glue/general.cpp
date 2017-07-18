#include <iostream>
#include <vector>
#include <string>
#include <algorithm>
#include <stdexcept>
#include <string.h>
#include "imports.h"
#include "types.h"

using namespace std;

extern "C" void ice_glue_destroy_header_iterator(Map<string, string>::iterator *itr_p) {
    delete itr_p;
}

extern "C" void ice_glue_destroy_cookie_iterator(Map<string, string>::iterator *itr_p) {
    delete itr_p;
}

typedef void (*AsyncEndpointHandler) (int id, void *call_info);
static AsyncEndpointHandler async_endpoint_handler = NULL;

extern "C" void ice_glue_register_async_endpoint_handler(AsyncEndpointHandler handler) {
    async_endpoint_handler = handler;
}

extern "C" void ice_glue_async_endpoint_handler(int id, void *call_info) {
    if(!async_endpoint_handler) throw runtime_error("Async endpoint handler not registered");

    async_endpoint_handler(id, call_info);
}
