use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use prefix_tree::PrefixTree;
//use sequence_trie::SequenceTrie;

pub struct Router {
    next_id: i32,
    routes: PrefixTree<String, Arc<RwLock<Endpoint>>>
}

pub struct Endpoint {
    pub id: i32,
    pub name: String,
    pub path: Vec<String>,
    pub param_names: Vec<String>,
    pub flags: HashMap<String, bool>
}

impl Router {
    pub fn new() -> Router {
        Router {
            next_id: 0,
            routes: PrefixTree::new()
        }
    }

    // Endpoints shouldn't be removed once added.
    pub fn add_endpoint(&mut self, p: &str) -> Arc<RwLock<Endpoint>> {
        let (path, param_names) = normalize_path(p);

        let ep = Arc::new(RwLock::new(Endpoint {
            id: self.next_id,
            name: p.to_string(),
            path: path.clone(),
            param_names: param_names,
            flags: HashMap::new()
        }));

        self.routes.insert(path.as_slice(), ep.clone());

        self.next_id += 1;
        
        ep
    }

    pub fn get_endpoint(&self, p: &str) -> Option<(Arc<RwLock<Endpoint>>, HashMap<String, String>)> {
        let (path, _) = normalize_path(p);
        match self.routes.find(path.as_slice(), Some(&":P".to_string())) {
            Some(rt) => {
                let params = {
                    let rt = rt.read().unwrap();
                    let mut p = HashMap::new();
                    let mut pn_pos: usize = 0;

                    for i in 0..rt.path.len() {
                        if rt.path[i].starts_with(":") {
                            p.insert(rt.param_names[pn_pos].clone(), path[i].clone());
                            pn_pos += 1;
                        }
                    }

                    p
                };
                Some((rt, params))
            },
            None => None
        }
    }
}

fn normalize_path(p: &str) -> (Vec<String>, Vec<String>) {
    let mut param_names = Vec::new();
    let path = p.split("/").filter(|v| v.len() > 0).map(|v| {
        if v.starts_with(":") {
            param_names.push(v[1..].to_string());
            ":P".to_string()
        } else {
            v.to_string()
        }
    }).collect();
    (path, param_names)
}
