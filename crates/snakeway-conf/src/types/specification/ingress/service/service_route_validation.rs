use super::service_issues;
use crate::types::{HclOrigin, ServiceRouteSpec};
use confval::{ValidateSpec, ValidationReport};

impl ValidateSpec<HclOrigin> for ServiceRouteSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        if self.hosts.is_empty() {
            report.push(service_issues::route_has_no_hosts(origin));
        }
    }
}
