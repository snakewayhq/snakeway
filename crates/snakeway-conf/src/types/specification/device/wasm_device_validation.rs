use crate::types::{Origin, WasmDeviceSpec};
use crate::validation::{ValidateSpec, ValidationReport};
use nix::NixPath;

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
