use crate::types::{Origin, RedirectSpec};
use crate::validation::validator::is_valid_port;
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(RESPONSE_CODE, u16, min: 300, max: 399);

impl ValidateSpec for RedirectSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        validate_range_field!(RESPONSE_CODE, self.status, report, origin);
    }
}
