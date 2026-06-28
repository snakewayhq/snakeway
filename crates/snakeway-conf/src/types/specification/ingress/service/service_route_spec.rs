use crate::types::HclInt;
use confval::prelude::{Located, Report, Span};
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
    pub ws_max_connections: Option<Located<HclInt>>,
}

pub(crate) fn validate_service_route(spec: &ServiceRouteSpec, span: Span, report: &mut Report) {
    if spec.hosts.is_empty() {
        report.error("route has no hosts").at(span).emit();
    }

    if let Some(ws_max_connections) = &spec.ws_max_connections {
        WS_MAX_CONNECTIONS.check_located(ws_max_connections, "ws_max_connections", report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_with_no_hosts_produces_error() {
        // Arrange
        let mut report = Report::new();
        let route = ServiceRouteSpec {
            hosts: vec![],
            path: Located::detached("/".to_string()),
            ..Default::default()
        };

        // Act
        validate_service_route(&route, Span::detached(), &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("route has no hosts"))
        );
    }

    #[test]
    fn ws_max_connections_below_minimum_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let route = ServiceRouteSpec {
            hosts: vec![Located::detached("example.com".to_string())],
            path: Located::detached("/".to_string()),
            ws_max_connections: Some(Located::detached(0)),
            ..Default::default()
        };

        // Act
        validate_service_route(&route, Span::detached(), &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("ws_max_connections"))
        );
    }

    #[test]
    fn ws_max_connections_within_range_is_accepted() {
        // Arrange
        let mut report = Report::new();
        let route = ServiceRouteSpec {
            hosts: vec![Located::detached("example.com".to_string())],
            path: Located::detached("/".to_string()),
            ws_max_connections: Some(Located::detached(1000)),
            ..Default::default()
        };

        // Act
        validate_service_route(&route, Span::detached(), &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }
}
