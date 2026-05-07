use crate::types::{HclOrigin, WasmDeviceSpec};
use confval::{ValidateSpec, ValidationReport};
use nix::NixPath;

impl ValidateSpec<HclOrigin> for WasmDeviceSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
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
