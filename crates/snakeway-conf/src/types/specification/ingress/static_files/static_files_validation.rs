use crate::types::{HclOrigin, StaticFilesSpec};
use confval::{ValidateSpec, ValidationReport};

impl ValidateSpec<HclOrigin> for StaticFilesSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        for route in &self.routes {
            route.validate(origin, report);
        }
    }
}
