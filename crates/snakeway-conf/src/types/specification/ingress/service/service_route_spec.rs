use crate::types::HclInt;
use confval::prelude::{Located, Report, Span};
use serde::Serialize;

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
}
