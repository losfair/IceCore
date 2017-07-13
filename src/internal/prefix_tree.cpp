#include <vector>
#include <string>
#include <map>

using namespace std;

const string param_indicator(":P");

static vector<string> split_string(const string& s, char delim, bool keep_empty);

class Endpoint {
    public:
        bool valid;
        int id;
        vector<string> param_names;

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

        void add_endpoint(const string& _path, int id) {
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
        }

        int get_endpoint_id(const string& _path) {
            PrefixTreeNode *current = root;
            vector<string> path = split_string(_path, '/', false);

            for(vector<string>::iterator itr_p = path.begin(); itr_p != path.end(); itr_p++) {
                const string& p = *itr_p;

                PrefixTreeNode *child = current -> get_child(p);
                if(!child) child = current -> get_child(param_indicator);
                if(!child) return -1;

                current = child;
            }

            if(!current -> ep.is_valid()) return -1;
            return current -> ep.id;
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

extern "C" void ice_internal_prefix_tree_add_endpoint(PrefixTree *t, const char *_name, int id) {
    const string name(_name);
    t -> add_endpoint(name, id);
}

extern "C" int ice_internal_prefix_tree_get_endpoint_id(PrefixTree *t, const char *_name) {
    const string name(_name);
    return t -> get_endpoint_id(name);
}