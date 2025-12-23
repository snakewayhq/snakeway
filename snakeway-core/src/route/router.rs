use crate::route::kind::RouteKind;
use anyhow::{Result, anyhow};

#[derive(Debug)]
pub struct Router {
    routes: Vec<RouteEntry>,
}

#[derive(Debug)]
pub struct RouteEntry {
    pub path: String,
    pub kind: RouteKind,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn add_route(&mut self, path: &str, kind: RouteKind) -> Result<()> {
        if !path.starts_with('/') {
            return Err(anyhow!("route path must start with '/': {}", path));
        }

        if self.routes.iter().any(|r| r.path == path) {
            return Err(anyhow!("duplicate route path: {}", path));
        }

        self.routes.push(RouteEntry {
            path: path.to_string(),
            kind,
        });

        // The longest prefix wins --> sort descending by path length.
        self.routes.sort_by(|a, b| b.path.len().cmp(&a.path.len()));

        Ok(())
    }

    pub fn match_route(&self, request_path: &str) -> Result<&RouteEntry> {
        if !request_path.starts_with('/') {
            return Err(anyhow!("invalid request path: {}", request_path));
        }

        for route in &self.routes {
            if path_matches(&route.path, request_path) {
                return Ok(route);
            }
        }

        Err(anyhow!("no route matched path {}", request_path))
    }

    pub(crate) fn route_count(&self) -> usize {
        self.routes.len()
    }
}

fn path_matches(route_path: &str, request_path: &str) -> bool {
    if route_path == "/" {
        return true;
    }

    if request_path == route_path {
        return true;
    }

    request_path.starts_with(route_path)
        && request_path
            .as_bytes()
            .get(route_path.len())
            .map(|b| *b == b'/')
            .unwrap_or(false)
}
