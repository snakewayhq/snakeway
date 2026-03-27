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

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range(
            self.max_requests_per_second,
            &REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
            report,
            origin,
        );
        validate_range(
            self.window_seconds,
            &REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS,
            report,
            origin,
        );
    }
}
