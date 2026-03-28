use crate::types::{Origin, RedirectSpec};
use crate::validation::validator::{REDIRECT_RESPONSE_CODE, is_valid_port};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for RedirectSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        REDIRECT_RESPONSE_CODE.validate(self.status, report, origin);
    }
}
