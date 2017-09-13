use futures::Future;
use hyper;
use prefix_tree::PrefixTree;
use http::server::path_utils;

pub type RouteCallbackFn = Fn(hyper::Request) -> Box<Future<Item = hyper::Response, Error = hyper::Error>> + Send + Sync;

pub struct HttpServerRoutingTable {
    routes: PrefixTree<String, RouteInfo>,
    default_route: Option<RouteInfo>
}

pub struct RouteInfo {
    method: hyper::Method,
    normalized_path: Vec<String>,
    param_names: Vec<String>,
    callback: Box<RouteCallbackFn>
}

impl HttpServerRoutingTable {
    pub fn new() -> HttpServerRoutingTable {
        HttpServerRoutingTable {
            routes: PrefixTree::new(),
            default_route: None
        }
    }

    pub fn add_route(&mut self, info: RouteInfo) {
        let normalized_path = info.normalized_path.clone();

        self.routes.insert(
            normalized_path.as_slice(),
            info
        )
    }

    pub fn set_default_route(&mut self, info: RouteInfo) {
        self.default_route = Some(info);
    }

    pub fn get_route(&self, path: &str) -> Option<&RouteInfo> {
        let (normalized_path, _) = path_utils::normalize_path(path, ":P");

        // Ugly clones.
        let normalized_path: Vec<String> = normalized_path
            .iter()
            .map(|v| v.to_string())
            .collect();

        if let Some(ref ret) = self.routes.find_ref(
            normalized_path.as_slice(),
            Some(&":P".to_string())
        ) {
            Some(ret)
        } else if let Some(ref ret) = self.default_route {
            Some(ret)
        } else {
            None
        }
    }
}

impl RouteInfo {
    pub fn new(path: &str, callback: Box<RouteCallbackFn>) -> RouteInfo {
        let (normalized_path, param_names) = path_utils::normalize_path(path, ":P");

        let normalized_path: Vec<String> = normalized_path
            .iter()
            .map(|v| v.to_string())
            .collect();

        let param_names: Vec<String> = param_names
            .iter()
            .map(|v| v.to_string())
            .collect();

        RouteInfo {
            method: hyper::Method::Get,
            normalized_path: normalized_path,
            param_names: param_names,
            callback: callback
        }
    }

    pub fn set_method(&mut self, method: hyper::Method) {
        self.method = method;
    }

    pub fn call(&self, req: hyper::Request) -> Box<Future<Item = hyper::Response, Error = hyper::Error>> {
        (self.callback)(req)
    }
}
