use super::device_issues;
use crate::types::{HclOrigin, WasmDeviceSpec};
use confval::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};
use nix::NixPath;

range_constraint!(TIMEOUT_MS, i64, min: 1, max: 60000, units: "milliseconds");
range_constraint!(BODY_BUFFER_MAX, i64, min: 0, max: 104857600);

impl ValidateSpec<HclOrigin> for WasmDeviceSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        if self.name.trim().is_empty() {
            report.push(device_issues::wasm_device_name_is_empty(origin));
        }

        validate_range_field!(TIMEOUT_MS, self.timeout_ms, report, origin);
        validate_range_field!(BODY_BUFFER_MAX, self.body_buffer_max, report, origin);

        if self.path.is_empty() {
            report.push(device_issues::wasm_device_path_is_empty(
                self.path.display(),
                origin,
            ));
        }
        if !self.path.exists() {
            report.push(device_issues::wasm_device_path_does_not_exist(
                self.path.display(),
                origin,
            ));
        }
        if !self.path.is_file() {
            report.push(device_issues::wasm_device_path_is_not_a_file(
                self.path.display(),
                origin,
            ));
        }
    }
}
