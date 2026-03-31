use crate::types::ServiceRouteSpec;

use super::ServiceRouteConfig;

impl ServiceRouteConfig {
    pub fn new(service: &str, listener: &str, spec: ServiceRouteSpec) -> Self {
        Self {
            service: service.to_string(),
            listener: listener.to_string(),
            hosts: spec.hosts,
            path: spec.path,
            allow_websocket: spec.enable_websocket,
            ws_max_connections: spec.ws_max_connections,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Origin;

    #[test]
    fn new_maps_fields_correctly() {
        // Arrange
        let spec = ServiceRouteSpec {
            origin: Origin::default(),
            hosts: vec!["example.com".to_string()],
            path: "/api".to_string(),
            enable_websocket: true,
            ws_max_connections: Some(100),
        };

        // Act
        let config = ServiceRouteConfig::new("my-service", "my-listener", spec);

        // Assert
        assert_eq!(config.service, "my-service");
        assert_eq!(config.listener, "my-listener");
        assert_eq!(config.hosts, vec!["example.com"]);
        assert_eq!(config.path, "/api");
        assert!(config.allow_websocket);
        assert_eq!(config.ws_max_connections, Some(100));
    }
}
