use crate::execution::route::types::RouteRuntime;
use anyhow::{Result, anyhow};

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

impl From<String> for HostMatcher {
    fn from(host: String) -> Self {
        let host = host.to_ascii_lowercase();
        if host == "*" {
            HostMatcher::Any
        } else if host.starts_with("*.") {
            HostMatcher::Wildcard(host)
        } else {
            HostMatcher::Exact(host)
        }
    }
}

#[derive(Debug)]
pub struct RouteEntry {
    pub(crate) hosts: Vec<HostMatcher>,
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

        // Config validation in snakeway-conf should prevent duplicate route
        // paths within the same listener. This check is defense in depth.
        if self.routes.iter().any(|r| r.path == path) {
            return Err(anyhow!("duplicate route path: {}", path));
        }

        let hosts = hosts.into_iter().map(HostMatcher::from).collect();

        self.routes.push(RouteEntry {
            hosts,
            path: path.to_string(),
            kind,
        });

        // The longest prefix wins --> sort descending by path length.
        self.routes.sort_by_key(|b| std::cmp::Reverse(b.path.len()));

        Ok(())
    }

    pub fn match_route(&self, host: &str, request_path: &str) -> Result<&RouteEntry> {
        if !request_path.starts_with('/') {
            return Err(anyhow!("invalid request path: {}", request_path));
        }

        for route in &self.routes {
            if path_matches_prefix(&route.path, request_path) && route_matches_host(host, route) {
                return Ok(route);
            }
        }

        Err(anyhow!("no route matched path {}", request_path))
    }
}

/// Returns `true` if the request path matches at least one of the given
/// path prefixes. Callers should skip this check entirely when `scopes`
/// is empty (meaning the device applies to all paths).
pub(crate) fn request_path_in_scope(scopes: &[String], request_path: &str) -> bool {
    scopes.iter().any(|p| path_matches_prefix(p, request_path))
}

/// Sorts path prefixes by descending length so that longer (more specific)
/// prefixes are tested first during matching.
pub(crate) fn sort_paths_longest_first(paths: &mut [String]) {
    paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
}

/// Tests whether a request path falls under the given prefix using
/// slash-boundary-aware prefix matching.
///
/// Returns `true` when:
/// - `prefix` is `"/"` (matches everything),
/// - `request_path` equals `prefix` exactly, or
/// - `request_path` starts with `prefix` and the next character is `"/"`.
///
/// The slash boundary check prevents `/api` from matching `/apikeys`.
pub(crate) fn path_matches_prefix(prefix: &str, request_path: &str) -> bool {
    if prefix == "/" {
        return true;
    }

    if request_path == prefix {
        return true;
    }

    request_path.starts_with(prefix)
        && request_path
            .as_bytes()
            .get(prefix.len())
            .map(|b| *b == b'/')
            .unwrap_or(false)
}

fn route_matches_host(host: &str, route: &RouteEntry) -> bool {
    route
        .hosts
        .iter()
        .any(|matcher| host_matches(matcher, host))
}

fn host_matches(matcher: &HostMatcher, host: &str) -> bool {
    match matcher {
        HostMatcher::Exact(h) => h.eq_ignore_ascii_case(host),

        HostMatcher::Wildcard(pattern) => {
            // Stored as "*.example.com" — strip the wildcard prefix to get the base domain.
            let Some(suffix) = pattern.strip_prefix("*.") else {
                return false;
            };

            // The request host must end with ".example.com" (dot-anchored to prevent
            // "xnotexample.com" from matching "*.example.com").
            let ends_with_suffix = host.ends_with(&format!(".{suffix}"));

            // Wildcards cover exactly one label (RFC 6125).
            // "foo.example.com" matches, but "deep.sub.example.com" does not.
            let expected_label_count = suffix.split('.').count() + 1;
            let actual_label_count = host.split('.').count();
            let is_single_label_wildcard = actual_label_count == expected_label_count;

            ends_with_suffix && is_single_label_wildcard
        }

        HostMatcher::Any => true,
    }
}
