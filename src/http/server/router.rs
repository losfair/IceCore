use futures::Future;
use hyper;
use prefix_tree::PrefixTree;
use http::server::path_utils;

pub type RouteCallbackFn = Fn(hyper::Request) -> Box<Future<Item = hyper::Response, Error = hyper::Error>>;

pub struct HttpServerRoutingTable {
    routes: PrefixTree<String, RouteInfo>
}

pub struct RouteInfo {
    pub method: hyper::Method,
    pub normalized_path: Vec<String>,
    pub param_names: Vec<String>,
    pub callback: Box<RouteCallbackFn>
}

impl HttpServerRoutingTable {
    pub fn new() -> HttpServerRoutingTable {
        HttpServerRoutingTable {
            routes: PrefixTree::new()
        }
    }

    pub fn add_route(&mut self, path: &str, info: RouteInfo) {
        let normalized_path = info.normalized_path.clone();

        self.routes.insert(
            normalized_path.as_slice(),
            info
        )
    }

    pub fn get_route(&self, path: &str) -> Option<&RouteInfo> {
        let (normalized_path, _) = path_utils::normalize_path(path, ":P");

        // Ugly clones.
        let normalized_path: Vec<String> = normalized_path
            .iter()
            .map(|v| v.to_string())
            .collect();

        self.routes.find_ref(
            normalized_path.as_slice(),
            Some(&":P".to_string())
        )
    }
}
