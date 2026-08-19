use confval::prelude::{Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(WS_MAX_CONNECTIONS, i64, min: 1, max: 1_048_576);

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct ServiceRouteSpec {
    pub hosts: Vec<Located<String>>,
    pub path: Located<String>,
    #[confval(default)]
    pub enable_websocket: Located<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_max_connections: Option<Located<i64>>,
}

/// The "route has no hosts" rule reports at the route's enclosing span, which
/// an empty `hosts` list cannot supply, so it lives in `ServiceSpec`.
impl Validate for ServiceRouteSpec {
    fn validate(&self, report: &mut Report) {
        if let Some(ws_max_connections) = &self.ws_max_connections {
            WS_MAX_CONNECTIONS.check_located(ws_max_connections, "ws_max_connections", report);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_max_connections_below_minimum_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let route = ServiceRouteSpec {
            hosts: vec![Located::detached("example.com".to_string())],
            path: Located::detached("/".to_string()),
            enable_websocket: Located::detached(true),
            ws_max_connections: Some(Located::detached(0)),
        };

        // Act
        route.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("ws_max_connections"))
        );
    }

    #[test]
    fn ws_max_connections_checked_when_websocket_disabled() {
        // Arrange
        let mut report = Report::new();
        let route = ServiceRouteSpec {
            hosts: vec![Located::detached("example.com".to_string())],
            path: Located::detached("/".to_string()),
            enable_websocket: Located::detached(false),
            ws_max_connections: Some(Located::detached(0)),
        };

        // Act
        route.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("ws_max_connections")),
            "a present ws_max_connections must be validated even when websocket is off; issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn ws_max_connections_within_range_is_accepted() {
        // Arrange
        let mut report = Report::new();
        let route = ServiceRouteSpec {
            hosts: vec![Located::detached("example.com".to_string())],
            path: Located::detached("/".to_string()),
            enable_websocket: Located::detached(true),
            ws_max_connections: Some(Located::detached(1000)),
        };

        // Act
        route.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }
}
