use crate::types::{Origin, StaticFilesSpec};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for StaticFilesSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        self.routes.map(|route| route.validate(origin, report));
    }
}
