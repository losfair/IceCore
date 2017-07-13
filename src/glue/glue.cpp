#include <vector>
#include <string>
#include <map>
#include <algorithm>
#include <string.h>

using namespace std;

typedef unsigned int u32;
typedef unsigned char u8;

class HasHeader {
    public:
        virtual void add_header(const char *key, const char *value) = 0;
        virtual const string& get_header(const char *key) = 0;
        virtual map<string, string>::iterator get_header_iterator_begin() = 0;
        virtual map<string, string>::iterator get_header_iterator_end() = 0;
};

class Request : public HasHeader {
    public:
        string remote_addr;
        string method;
        string uri;
        map<string, string> headers;

        Request() {}

        void set_remote_addr(const char *addr) {
            remote_addr = addr;
        }

        void set_method(const char *_m) {
            method = _m;
        }

        void set_uri(const char *_uri) {
            uri = _uri;
        }

        virtual void add_header(const char *key, const char *value) {
            string lower_key = key;
            transform(lower_key.begin(), lower_key.end(), lower_key.begin(), ::tolower);

            headers[lower_key] = value;
        }

        virtual const string& get_header(const char *key) {
            string lower_key = key;
            transform(lower_key.begin(), lower_key.end(), lower_key.begin(), ::tolower);

            return headers[lower_key];
        }

        virtual map<string, string>::iterator get_header_iterator_begin() {
            return headers.begin();
        }

        virtual map<string, string>::iterator get_header_iterator_end() {
            return headers.end();
        }
};

extern "C" Request * ice_glue_create_request() {
    return new Request();
}

extern "C" void ice_glue_destroy_request(Request *req) {
    delete req;
}

extern "C" void ice_glue_request_set_remote_addr(Request *req, const char *addr) {
    req -> set_remote_addr(addr);
}

extern "C" void ice_glue_request_set_method(Request *req, const char *m) {
    req -> set_method(m);
}

extern "C" void ice_glue_request_set_uri(Request *req, const char *uri) {
    req -> set_uri(uri);
}

extern "C" const char * ice_glue_request_get_remote_addr(Request *req) {
    return req -> remote_addr.c_str();
}

extern "C" const char * ice_glue_request_get_method(Request *req) {
    return req -> method.c_str();
}

extern "C" const char * ice_glue_request_get_uri(Request *req) {
    return req -> uri.c_str();
}

class Response : public HasHeader {
    public:
        map<string, string> headers;
        string body;

        Response() {}

        virtual void add_header(const char *key, const char *value) {
            string lower_key = key;
            transform(lower_key.begin(), lower_key.end(), lower_key.begin(), ::tolower);

            headers[lower_key] = value;
        }

        virtual const string& get_header(const char *key) {
            string lower_key = key;
            transform(lower_key.begin(), lower_key.end(), lower_key.begin(), ::tolower);

            return headers[lower_key];
        }

        virtual map<string, string>::iterator get_header_iterator_begin() {
            return headers.begin();
        }

        virtual map<string, string>::iterator get_header_iterator_end() {
            return headers.end();
        }

        void set_body(const u8 *_body, u32 len) {
            body = string((const char *) _body, len);
        }

        const u8 * get_body(u32 *len_out) {
            if(len_out) *len_out = body.size();
            return (const u8 *) &body[0];
        }
};

extern "C" Response * ice_glue_create_response() {
    return new Response();
}

extern "C" void ice_glue_destroy_response(Response *resp) {
    delete resp;
}

extern "C" void ice_glue_response_set_body(Response *resp, const u8 *body, u32 len) {
    resp -> set_body(body, len);
}

extern "C" void ice_glue_add_header(HasHeader *t, const char *k, const char *v) {
    t -> add_header(k, v);
}

extern "C" const map<string, string>::iterator * ice_glue_create_header_iterator(HasHeader *t) {
    map<string, string>::iterator *itr_p = new map<string, string>::iterator();
    map<string, string>::iterator& itr = *itr_p;

    itr = t -> get_header_iterator_begin();
    return itr_p;
}

extern "C" void ice_glue_destroy_header_iterator(map<string, string>::iterator *itr_p) {
    delete itr_p;
}

extern "C" const char * ice_glue_header_iterator_next(HasHeader *t, map<string, string>::iterator *itr_p) {
    map<string, string>::iterator& itr = *itr_p;
    if(itr == t -> get_header_iterator_end()) return NULL;

    const char *ret = itr -> first.c_str();
    itr++;

    return ret;
}

extern "C" const char * ice_glue_get_header(HasHeader *t, const char *k) {
    return t -> get_header(k).c_str();
}

extern "C" const u8 * ice_glue_response_get_body(Response *resp, u32 *len_out) {
    return &(resp -> get_body(len_out)[0]);
}

typedef Response * (*EndpointHandler) (const char *, Request *);
static EndpointHandler endpoint_handler = NULL;

extern "C" Response * ice_glue_endpoint_handler(const char *server_id, Request *req) {
    if(!endpoint_handler) {
        return ice_glue_create_response();
    }
    return endpoint_handler(server_id, req);
}

extern "C" void ice_glue_register_endpoint_handler(EndpointHandler handler) {
    endpoint_handler = handler;
}
