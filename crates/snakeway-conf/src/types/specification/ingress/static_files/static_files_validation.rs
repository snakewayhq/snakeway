use crate::types::{Origin, StaticRouteSpec};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for StaticRouteSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if !self.file_dir.exists() {
            report.invalid_static_dir(&self.file_dir, origin);
        }
        if self.file_dir.is_relative() {
            report.invalid_static_dir_must_be_absolute(&self.file_dir, origin);
        }
    }
}
