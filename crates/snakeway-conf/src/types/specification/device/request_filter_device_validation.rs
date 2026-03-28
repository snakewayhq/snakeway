use crate::types::{Origin, RequestFilterDeviceSpec};
use crate::validation::validator::{
    REQUEST_FILTER_DENY_STATUS, validate_http_header_name, validate_http_method,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for RequestFilterDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if let Some(deny_status) = self.deny_status {
            REQUEST_FILTER_DENY_STATUS.validate(deny_status, report, origin);
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
