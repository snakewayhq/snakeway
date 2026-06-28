use crate::types::ServiceRouteSpec;
use confval::prelude::{Report, narrow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceRouteConfig {
    /// Host names allowed to access this route.
    pub hosts: Vec<String>,

    /// Path prefix (longest-prefix match).
    pub path: String,

    pub service: String,

    pub allow_websocket: bool,
    pub ws_max_connections: Option<usize>,

    pub listener: String,
}

impl ServiceRouteConfig {
    pub fn new(
        service: &str,
        listener: &str,
        spec: &ServiceRouteSpec,
        report: &mut Report,
    ) -> Option<Self> {
        Some(Self {
            service: service.to_string(),
            listener: listener.to_string(),
            hosts: spec.hosts.iter().map(|h| h.value.clone()).collect(),
            path: spec.path.value.clone(),
            allow_websocket: spec.enable_websocket.value,
            ws_max_connections: narrow::opt_i64_to_usize(&spec.ws_max_connections, report)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::Located;

    #[test]
    fn new_maps_fields_correctly() {
        // Arrange
        let spec = ServiceRouteSpec {
            hosts: vec![Located::detached("example.com".to_string())],
            path: Located::detached("/api".to_string()),
            enable_websocket: Located::detached(true),
            ws_max_connections: Some(Located::detached(100)),
        };
        let mut report = Report::new();

        // Act
        let config =
            ServiceRouteConfig::new("my-service", "my-listener", &spec, &mut report).unwrap();

        // Assert
        assert!(!report.has_errors());
        assert_eq!(config.service, "my-service");
        assert_eq!(config.listener, "my-listener");
        assert_eq!(config.hosts, vec!["example.com"]);
        assert_eq!(config.path, "/api");
        assert!(config.allow_websocket);
        assert_eq!(config.ws_max_connections, Some(100));
    }

    #[test]
    fn negative_ws_max_connections_is_reported_not_wrapped() {
        // Arrange: lowering gates on a clean report, but the narrow is a
        // backstop against a negative value wrapping into a huge usize.
        let spec = ServiceRouteSpec {
            hosts: vec![Located::detached("example.com".to_string())],
            path: Located::detached("/api".to_string()),
            enable_websocket: Located::detached(true),
            ws_max_connections: Some(Located::detached(-1)),
        };
        let mut report = Report::new();

        // Act
        let config = ServiceRouteConfig::new("my-service", "my-listener", &spec, &mut report);

        // Assert
        assert!(config.is_none());
        assert!(report.has_errors());
    }
}
