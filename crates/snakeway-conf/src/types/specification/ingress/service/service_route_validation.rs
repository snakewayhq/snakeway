use crate::types::{Origin, ServiceRouteSpec};
use crate::validation::{ValidateSpec, ValidationReportDeprecated};

impl ValidateSpec for ServiceRouteSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated) {
        if self.hosts.is_empty() {
            report.route_has_no_hosts(origin);
        }
    }
}
