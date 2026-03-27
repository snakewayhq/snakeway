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

impl ValidateSpec for WasmDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.path.is_empty() {
            report.wasm_device_path_is_empty(self.path.display(), origin);
        }
        if !self.path.exists() {
            report.wasm_device_path_does_not_exist(self.path.display(), origin);
        }
        if !self.path.is_file() {
            report.wasm_device_path_is_not_a_file(self.path.display(), origin);
        }
    }
}
