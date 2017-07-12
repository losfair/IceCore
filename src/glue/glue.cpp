#include <vector>
#include <string>
#include <vector>
#include <string.h>

using namespace std;

typedef unsigned int u32;
typedef unsigned char u8;

class Header {
    public:
        string key, value;

        Header(const char *_key, const char *_value) {
            key = _key;
            value = _value;
        }
};

class RawLayout {
    public:
        vector<u8> raw;

        RawLayout() {

        }

        ~RawLayout() {

        }

        RawLayout& add(const u8 *p, u32 len) {
            add_size(len);

            for(u32 i = 0; i < len; i++) {
                raw.push_back(p[i]);
            }

            return *this;
        }

        RawLayout& add(const string& s) {
            return add((const u8 *) s.c_str(), s.size());
        }

        RawLayout& add_size(u32 v) {
            for(u32 i = 0; i < sizeof(u32); i++) {
                raw.push_back(((u8 *)(&v))[i]);
            }

            return *this;
        }

        u8 * to_raw() {
            u8 *ret = new u8 [raw.size()];
            memcpy(ret, &raw[0], raw.size());
            return ret;
        }

        u32 size() {
            return raw.size();
        }
};

class Request {
    public:
        string remote_addr;
        string method;
        string uri;
        vector<Header> headers;

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

        void add_header(const char *key, const char *value) {
            headers.push_back(Header(key, value));
        }

        /*
        Layout:
        - remote_addr_len (4)
        - remote_addr (remote_addr_len)
        - method_len (4)
        - method (method_len)
        - uri_len (4)
        - uri (uri_len)
        - headers_count (4)
        - header:
            - key_len (4)
            - key (key_len)
            - value_len(4)
            - value(value_len)
        */
        u8 * to_raw() {
            RawLayout layout;

            layout
            .add(remote_addr)
            .add(method)
            .add(uri)
            .add_size(headers.size());

            for(vector<Header>::iterator hdr = headers.begin(); hdr != headers.end(); hdr++) {
                layout.add(hdr -> key).add(hdr -> value);
            }

            return layout.to_raw();
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

extern "C" void ice_glue_request_add_header(Request *req, const char *k, const char *v) {
    req -> add_header(k, v);
}

extern "C" u8 * ice_glue_request_to_raw(Request *req) {
    return req -> to_raw();
}

extern "C" void * ice_glue_request_destroy_raw(u8 *raw) {
    delete[] raw;
}
