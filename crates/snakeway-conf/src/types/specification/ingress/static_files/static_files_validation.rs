use crate::types::{OriginDeprecated, StaticFilesSpec};
use crate::validation::{ValidateSpec, ValidationReportDeprecated};

impl ValidateSpec for StaticFilesSpec {
    fn validate(&self, origin: &OriginDeprecated, report: &mut ValidationReportDeprecated) {
        for route in &self.routes {
            route.validate(origin, report);
        }
    }
}
