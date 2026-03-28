use crate::types::{Origin, RequestFilterDeviceSpec};
use crate::validation::validator::{validate_http_header_name, validate_http_method};
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport, range_constraint};

range_constraint!(DENY_STATUS, u16, min: 400, max: 599, label: "request_filter_device.deny_status");

impl ValidateSpec for RequestFilterDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if let Some(deny_status) = self.deny_status {
            DENY_STATUS.validate(deny_status, report, origin);
        }

        for method in &self.allow_methods {
            validate_http_method(method, report, origin);
        }

        for method in &self.deny_methods {
            validate_http_method(method, report, origin);
        }

        for header in &self.deny_headers {
            validate_http_header_name(header, report, origin);
        }

        for header in &self.allow_headers {
            validate_http_header_name(header, report, origin);
        }

        for header in &self.required_headers {
            validate_http_header_name(header, report, origin);
        }
    }
}
