use crate::route::types::RouteRuntime;
use anyhow::{Result, anyhow};
use std::str::FromStr;

#[derive(Debug)]
pub struct Router {
    routes: Vec<RouteEntry>,
}

#[derive(Debug, Clone)]
pub enum HostMatcher {
    Exact(String),
    /// "*.example.com"
    Wildcard(String),
    /// "*"
    Any,
}

impl FromStr for HostMatcher {
    type Err = anyhow::Error;
    fn from_str(host: &str) -> Result<Self, Self::Err> {
        if host == "*" {
            Ok(HostMatcher::Any)
        } else if host.starts_with("*.") {
            Ok(HostMatcher::Wildcard(host.to_ascii_lowercase()))
        } else {
            Ok(HostMatcher::Exact(host.to_ascii_lowercase()))
        }
    }
}

#[derive(Debug)]
pub struct RouteEntry {
    pub hosts: Vec<HostMatcher>,
    pub path: String,
    pub kind: RouteRuntime,
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

    pub fn add_route(&mut self, hosts: Vec<String>, path: &str, kind: RouteRuntime) -> Result<()> {
        if !path.starts_with('/') {
            return Err(anyhow!("route path must start with '/': {}", path));
        }

        if self.routes.iter().any(|r| r.path == path) {
            return Err(anyhow!("duplicate route path: {}", path));
        }

        let hosts = hosts
            .into_iter()
            .map(|h: String| parse_host(h.as_str()))
            .collect();

        self.routes.push(RouteEntry {
            hosts,
            path: path.to_string(),
            kind,
        });

        // The longest prefix wins --> sort descending by path length.
        self.routes.sort_by(|a, b| b.path.len().cmp(&a.path.len()));

        Ok(())
    }

    pub fn match_route(&self, host: &str, request_path: &str) -> Result<&RouteEntry> {
        if !request_path.starts_with('/') {
            return Err(anyhow!("invalid request path: {}", request_path));
        }

        for route in &self.routes {
            if path_matches(&route.path, request_path) && route_matches_host(host, route) {
                return Ok(route);
            }
        }

        Err(anyhow!("no route matched path {}", request_path))
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

fn route_matches_host(host: &str, route: &RouteEntry) -> bool {
    route
        .hosts
        .iter()
        .any(|matcher| host_matches(matcher, host))
}

/// todo move this deeper into config subsystem
fn parse_host(host: &str) -> HostMatcher {
    if host == "*" {
        HostMatcher::Any
    } else if host.starts_with("*.") {
        HostMatcher::Wildcard(host.to_ascii_lowercase())
    } else {
        HostMatcher::Exact(host.to_ascii_lowercase())
    }
}

fn host_matches(matcher: &HostMatcher, host: &str) -> bool {
    match matcher {
        HostMatcher::Exact(h) => h.eq_ignore_ascii_case(host),

        HostMatcher::Wildcard(pattern) => {
            // pattern = "*.example.com"
            if let Some(stripped) = pattern.strip_prefix("*.") {
                host.ends_with(stripped) && host.split('.').count() > stripped.split('.').count()
            } else {
                false
            }
        }

        HostMatcher::Any => true,
    }
}
