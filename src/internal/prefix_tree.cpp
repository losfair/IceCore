#include <vector>
#include <string>
#include <map>
#include <stdexcept>

using namespace std;

const string param_indicator(":P");

static vector<string> split_string(const string& s, char delim, bool keep_empty);

class Endpoint {
    public:
        bool valid;
        int id;
        vector<string> param_names;
        map<string, bool> flags;

        Endpoint() {
            valid = false;
            id = -1;
        }

        void set_id(int _id) {
            id = _id;
            valid = true;
        }

        bool is_valid() {
            return valid;
        }

        void set_flag(const string& name, bool value) {
            flags[name] = value;
        }

        bool get_flag(const string& name) {
            return flags[name];
        }
};

class PrefixTreeNode {
    public:
        Endpoint ep;
        map<string, PrefixTreeNode *> children;

        PrefixTreeNode() {
        }

        ~PrefixTreeNode() {
            for(map<string, PrefixTreeNode *>::iterator itr = children.begin(); itr != children.end(); itr++) {
                if(itr -> second) delete itr -> second;
            }
        }

        PrefixTreeNode * create_child(const string& name) {
            PrefixTreeNode *child = new PrefixTreeNode();

            if(children[name]) delete children[name];
            children[name] = child;

            return child;
        }

        PrefixTreeNode * get_child(const string& name) {
            return children[name];
        }

        PrefixTreeNode * get_or_create_child(const string& name) {
            PrefixTreeNode *ret = get_child(name);
            if(!ret) ret = create_child(name);
            return ret;
        }
};

class PrefixTree {
    public:
        PrefixTreeNode *root;

        PrefixTree() {
            root = new PrefixTreeNode();
        }

        ~PrefixTree() {
            delete root;
        }

        Endpoint * add_endpoint(const string& _path, int id) {
            PrefixTreeNode *current = root;
            vector<string> path = split_string(_path, '/', false);

            vector<string> param_names;

            for(vector<string>::iterator itr_p = path.begin(); itr_p != path.end(); itr_p++) {
                const string& p = *itr_p;
                if(p[0] == ':') {
                    param_names.push_back(p.substr(1));
                    current = current -> get_or_create_child(param_indicator);
                } else {
                    current = current -> get_or_create_child(p);
                }
            }

            current -> ep.set_id(id);
            current -> ep.param_names = param_names;

            return &(current -> ep);
        }

        Endpoint * get_endpoint(const string& _path) {
            PrefixTreeNode *current = root;
            vector<string> path = split_string(_path, '/', false);

            for(vector<string>::iterator itr_p = path.begin(); itr_p != path.end(); itr_p++) {
                const string& p = *itr_p;

                PrefixTreeNode *child = current -> get_child(p);
                if(!child) child = current -> get_child(param_indicator);
                if(!child) return NULL;

                current = child;
            }

            if(!current -> ep.is_valid()) return NULL;
            return &(current -> ep);
        }

        int get_endpoint_id(const string& path) {
            Endpoint *ep = get_endpoint(path);
            if(!ep) return -1;
            return ep -> id;
        }
};

static vector<string> split_string(const string& s, char delim, bool keep_empty) {
    vector<string> ret;
    string current("");

    for(int i = 0; i < s.size(); i++) {
        if(s[i] == delim) {
            if(keep_empty || (!keep_empty && !current.empty())) ret.push_back(current);
            current.clear();
            continue;
        }
        current += s[i];
    }

    if(keep_empty || (!keep_empty && !current.empty())) ret.push_back(current);
    return ret;
}

extern "C" PrefixTree * ice_internal_create_prefix_tree() {
    return new PrefixTree();
}

extern "C" void ice_internal_destroy_prefix_tree(PrefixTree *t) {
    delete t;
}

extern "C" Endpoint * ice_internal_prefix_tree_add_endpoint(PrefixTree *t, const char *_name, int id) {
    const string name(_name);
    return t -> add_endpoint(name, id);
}

extern "C" int ice_internal_prefix_tree_get_endpoint_id(PrefixTree *t, const char *_name) {
    const string name(_name);
    return t -> get_endpoint_id(name);
}

extern "C" Endpoint * ice_internal_prefix_tree_get_endpoint(PrefixTree *t, const char *_name) {
    const string name(_name);
    return t -> get_endpoint(name);
}

extern "C" int ice_internal_prefix_tree_endpoint_get_id(Endpoint *ep) {
    return ep -> id;
}

extern "C" bool ice_internal_prefix_tree_endpoint_get_flag(Endpoint *ep, const char *_name) {
    const string name(_name);
    return ep -> get_flag(name);
}

extern "C" void ice_internal_prefix_tree_endpoint_set_flag(Endpoint *ep, const char *_name, bool value) {
    const string name(_name);
    ep -> set_flag(name, value);
}

extern "C" vector<string>::iterator * ice_internal_prefix_tree_endpoint_create_param_name_iterator(Endpoint *ep) {
    vector<string>::iterator *itr_p = new vector<string>::iterator();
    vector<string>::iterator& itr = *itr_p;

    itr = ep -> param_names.begin();
    return itr_p;
}

extern "C" void ice_internal_prefix_tree_endpoint_destroy_param_name_iterator(vector<string>::iterator *itr_p) {
    delete itr_p;
}

extern "C" const char * ice_internal_prefix_tree_endpoint_param_name_iterator_next(Endpoint *ep, vector<string>::iterator *itr_p) {
    vector<string>::iterator& itr = *itr_p;

    if(itr == ep -> param_names.end()) return NULL;

    const char *name = itr -> c_str();
    itr++;
    
    return name;
}
