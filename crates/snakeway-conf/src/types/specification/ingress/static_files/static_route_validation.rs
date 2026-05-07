use crate::types::{Origin, StaticRouteSpec};
use crate::validation::{ValidateSpec, ValidationReportDeprecated};

impl ValidateSpec for StaticRouteSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated) {
        if !self.file_dir.exists() {
            report.invalid_static_dir(&self.file_dir, origin);
        }
        if self.file_dir.is_relative() {
            report.invalid_static_dir_must_be_absolute(&self.file_dir, origin);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{Origin, StaticRouteSpec};
    use crate::validation::{ValidateSpec, ValidationReportDeprecated};
    use std::path::PathBuf;

    #[test]
    fn static_file_dir_does_not_exist() {
        // Arrange
        let file_dir = "/non/existent/static";
        let expected_error = format!("invalid static directory: {}", file_dir);
        let mut report = ValidationReportDeprecated::default();
        let spec = StaticRouteSpec {
            file_dir: PathBuf::from(file_dir),
            ..Default::default()
        };
        let origin = Origin::test("static_files");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors.first().unwrap().message, expected_error);
    }

    #[test]
    fn static_file_dir_is_not_relative() {
        // Arrange
        let file_dir = "./www";
        let expected_error0 = format!("invalid static directory: {}", file_dir);
        let expected_error1 = format!(
            "static file directory must be an absolute path: {}",
            file_dir
        );
        let mut report = ValidationReportDeprecated::default();
        let spec = StaticRouteSpec {
            file_dir: PathBuf::from(file_dir),
            ..Default::default()
        };
        let origin = Origin::test("static_files");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors[0].message, expected_error0);
        assert_eq!(report.errors[1].message, expected_error1);
    }
}
