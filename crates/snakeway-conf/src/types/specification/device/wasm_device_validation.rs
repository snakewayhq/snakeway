use crate::types::{OriginDeprecated, WasmDeviceSpec};
use crate::validation::{ValidateSpec, ValidationReportDeprecated};
use nix::NixPath;

impl ValidateSpec for WasmDeviceSpec {
    fn validate(&self, origin: &OriginDeprecated, report: &mut ValidationReportDeprecated) {
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
