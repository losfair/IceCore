use std::collections::HashMap;

pub struct Router {
    next_route_id: u32,
    endpoint_mappings: Vec<EndpointCollection>
}

pub struct EndpointCollection {
    endpoints: HashMap<String, Endpoint>,
}

pub struct Endpoint {
    id: u32,
    param_names: Vec<String>
}

impl Router {
    pub fn new() -> Router {
        let mut endpoint_mappings: Vec<EndpointCollection> = Vec::with_capacity(65536);
        for _ in 0..65536 {
            endpoint_mappings.push(EndpointCollection {
                endpoints: HashMap::new()
            });
        }

        Router {
            next_route_id: 0,
            endpoint_mappings: endpoint_mappings
        }
    }

    pub fn add_endpoint(&mut self, p: &str) -> u32 {
        let (mask, desc, param_names) = parse_endpoint(p);
        println!("Mask: {}, Desc: {}, Param names: {:?}", mask, desc, param_names);

        self.endpoint_mappings[mask as usize].endpoints.insert(desc, Endpoint {
            id: self.next_route_id,
            param_names: param_names
        });
        self.next_route_id += 1;

        self.next_route_id - 1
    }
}

fn parse_endpoint(p: &str) -> (u16, String, Vec<String>) {
    let mut i: usize = 0;
    let mut mask: u16 = 0;
    let mut desc = String::new();
    let mut param_names: Vec<String> = Vec::new();

    for s in p.split("/") {
        if i >= 16 {
            panic!("Endpoint path too long");
        }
        if s.starts_with(":") {
            mask |= 1 << i;
            desc += ":P/";
            param_names.push(s.split_at(1).1.to_string());
        } else {
            desc += s;
            desc += "/";
        }
        i += 1;
    }

    (mask, desc, param_names)
}
