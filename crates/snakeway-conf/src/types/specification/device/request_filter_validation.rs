use crate::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, Origin, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, WasmDeviceSpec,
};
use crate::validation::report::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;
use crate::validation::validator::{
    IDENTITY_DEVICE_MAX_USER_AGENT_LENGTH, IDENTITY_DEVICE_MAX_X_FORWARDED_FOR_LENGTH,
    REQUEST_FILTER_DENY_STATUS, REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
    REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS, validate_http_header_name, validate_http_method,
    validate_range,
};
use ipnet::IpNet;
use nix::NixPath;
use std::net::IpAddr;
use std::path::Path;

impl ValidateSpec for RequestFilterDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if let Some(deny_status) = self.deny_status {
            validate_range(deny_status, &REQUEST_FILTER_DENY_STATUS, report, origin);
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
