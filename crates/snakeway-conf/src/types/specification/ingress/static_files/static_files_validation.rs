use crate::types::{Origin, StaticFilesSpec};
use crate::validation::{ValidateSpec, ValidationReportDeprecated};

impl ValidateSpec for StaticFilesSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated) {
        for route in &self.routes {
            route.validate(origin, report);
        }
    }
}
