use crate::types::{Origin, RedirectSpec};
use crate::validation::validator::is_valid_port;
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport, range_constraint};

range_constraint!(RESPONSE_CODE, u16, min: 300, max: 399, label: "redirect_response_code");

impl ValidateSpec for RedirectSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        RESPONSE_CODE.validate(self.status, report, origin);
    }
}
