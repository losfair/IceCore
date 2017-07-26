use std::collections::HashMap;
use sequence_trie::SequenceTrie;

pub struct Router {
    next_id: i32,
    routes: SequenceTrie<String, Endpoint>
}

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub id: i32,
    pub name: String,
    pub param_names: Vec<String>,
    pub flags: HashMap<String, bool>
}

impl Router {
    pub fn new() -> Router {
        Router {
            next_id: 0,
            routes: SequenceTrie::new()
        }
    }

    // Endpoints shouldn't be removed once added.
    pub fn add_endpoint(&mut self, p: &str) -> *mut Endpoint {
        let (path, param_names) = normalize_path(p);

        self.routes.insert(&path, Endpoint {
            id: self.next_id,
            name: p.to_string(),
            param_names: param_names,
            flags: HashMap::new()
        });
        let ep = self.routes.get_mut(&path).unwrap() as *mut Endpoint; // Dangerous.

        self.next_id += 1;
        
        ep
    }

    pub fn borrow_endpoint(&self, p: &str) -> Option<&Endpoint> {
        let (_path, _) = normalize_path(p);
        let mut current = &self.routes;
        let mut path = _path.as_slice();
        let mut to_add = 1;

        loop {
            //println!("Path: {:?}", path);
            let nodes = current.get_prefix_nodes(path);
            //println!("Nodes: {:?}", nodes);

            // FIXME: This is too hacky
            if nodes.len() == path.len() + to_add {
                match nodes[nodes.len() - 1].value() {
                    Some(v) => return Some(v),
                    None => {}
                }
            }
            let next = nodes[nodes.len() - 1].get_prefix_nodes(&[":P".to_string()]);
            //println!("Next: {:?}", next);
            if next.len() <= 1 {
                return None;
            }
            current = next[1];
            to_add = 0;
            path = &path[(nodes.len() - 1)..];
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
