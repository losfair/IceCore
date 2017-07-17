#include <iostream>
#include <vector>
#include <string>
#include <algorithm>
#include <stdexcept>
#include <string.h>
#include "imports.h"
#include "types.h"

using namespace std;

class Response {
    public:
        Map<string, string> headers;
        string body;
        u16 status_code;

        Response() {
            status_code = 200;
        }

        void add_header(const char *key, const char *value) {
            string lower_key = key;
            transform(lower_key.begin(), lower_key.end(), lower_key.begin(), ::tolower);

            headers[lower_key] = value;
        }

        const string& get_header(const char *key) {
            string lower_key = key;
            transform(lower_key.begin(), lower_key.end(), lower_key.begin(), ::tolower);

            return headers[lower_key];
        }

        Map<string, string>::iterator get_header_iterator_begin() {
            return headers.begin();
        }

        Map<string, string>::iterator get_header_iterator_end() {
            return headers.end();
        }

        void set_body(const u8 *_body, u32 len) {
            body = string((const char *) _body, len);
        }

        const u8 * get_body(u32 *len_out) {
            //cerr << "get_body() begin" << endl;
            if(len_out) *len_out = body.size();
            //cerr << "len_out done" << endl;

            if(body.size() == 0) return NULL;
            else return (const u8 *) &body[0];
        }

        void set_status(u16 _status) {
            if(_status < 100 || _status >= 600) {
                return;
            }

            status_code = _status;
        }

        u16 get_status() const {
            return status_code;
        }
};

extern "C" Response * ice_glue_create_response() {
    return new Response();
}

extern "C" void ice_glue_destroy_response(Response *resp) {
    delete resp;
}

extern "C" void ice_glue_response_add_header(Response *t, const char *k, const char *v) {
    t -> add_header(k, v);
}

extern "C" Map<string, string>::iterator * ice_glue_response_create_header_iterator(Response *t) {
    Map<string, string>::iterator *itr_p = new Map<string, string>::iterator();
    Map<string, string>::iterator& itr = *itr_p;

    itr = t -> get_header_iterator_begin();
    return itr_p;
}

extern "C" const char * ice_glue_response_header_iterator_next(Response *t, Map<string, string>::iterator *itr_p) {
    Map<string, string>::iterator& itr = *itr_p;
    if(itr == t -> get_header_iterator_end()) return NULL;

    const char *ret = itr -> first.c_str();
    itr++;

    return ret;
}

extern "C" const char * ice_glue_response_get_header(Response *t, const char *k) {
    return t -> get_header(k).c_str();
}

extern "C" const u8 * ice_glue_response_get_body(Response *t, u32 *len_out) {
    //cerr << "ice_glue_get_body(" << t << ")" << endl;
    return t -> get_body(len_out);
}

extern "C" void ice_glue_response_set_body(Response *t, const u8 *body, u32 len) {
    t -> set_body(body, len);
}

extern "C" void ice_glue_response_set_status(Response *t, u16 status) {
    t -> set_status(status);
}

extern "C" u16 ice_glue_response_get_status(Response *t) {
    return t -> get_status();
}
