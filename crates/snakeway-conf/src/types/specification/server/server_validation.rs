use crate::types::{Origin, ServerSpec};
use crate::validation::validator::validate_cert_pem;
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(THREADS, usize, min: 1, max: 1024);
range_constraint!(DNS_REFRESH_INTERVAL_SECONDS, u64, min: 1, max: 3600, units: "seconds");

impl ValidateSpec for ServerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.version != 1 {
            report.invalid_config_version(&self.version, origin);
        }

        if let Some(pid_file) = self.pid_file.clone() {
            let Some(parent) = pid_file.parent() else {
                return;
            };

            if !parent.exists() {
                report.pid_file_parent_dir_does_not_exist(pid_file.display(), origin);
            } else if !parent.is_dir() {
                report.pid_file_parent_not_a_dir(pid_file.display(), origin);
            }
        }

        if let Some(ca_file) = &self.ca_file
            && let Err(e) = validate_cert_pem(ca_file)
        {
            report.server_ca_file_invalid(&e, origin);
        }

        if let Some(threads) = self.threads {
            validate_range_field!(THREADS, threads, report, origin);
        }

        validate_range_field!(
            DNS_REFRESH_INTERVAL_SECONDS,
            self.dns_refresh_interval_seconds,
            report,
            origin
        );

        if let Some(tls_automation) = &self.tls_automation {
            tls_automation.validate(origin, report);
        }

        if let Some(observability) = &self.observability {
            observability.validate(origin, report);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::ServerSpec;
    use crate::validation::{ValidateSpec, ValidationReport};
    use std::path::PathBuf;

    #[test]
    fn validate_server_version_valid() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            version: 1,
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn validate_server_version_invalid() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            version: 2,
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report.errors[0]
                .message
                .contains("invalid config version: 2")
        );
    }

    #[test]
    fn validate_server_valid_config() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            version: 1,
            threads: Some(4),
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn validate_server_pid_file_parent_dir_does_not_exist() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            pid_file: Some(PathBuf::from("/non/existent/path/snakeway.pid")),
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report.errors[0]
                .message
                .contains("pid file parent directory does not exist")
        );
    }

    #[test]
    fn validate_server_ca_file_does_not_exist() {
        // Arrange
        let ca_file = PathBuf::from("/non/existent/ca.pem");
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            ca_file: Some(ca_file.clone()),
            ..Default::default()
        };
        let expected = format!(
            "server CA file is invalid: file does not exist: {}",
            ca_file.to_string_lossy()
        );

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors[0].message.contains(&expected));
    }

    #[test]
    fn validate_server_threads_too_low() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            threads: Some(0),
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors[0].message.contains("invalid threads: 0"));
    }

    #[test]
    fn validate_server_threads_too_high() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            threads: Some(1025),
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors[0].message.contains("invalid threads: 1025"));
    }

    #[test]
    fn validate_server_pid_file_parent_is_not_a_dir() {
        // Arrange
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();

        // Create a file that will be used as the "parent"
        let fake_parent = dir.path().join("not_a_dir");
        std::fs::write(&fake_parent, "hello").unwrap();

        let server = ServerSpec {
            pid_file: Some(fake_parent.join("snakeway.pid")),
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("pid file parent is not a directory"))
        );
    }

    #[test]
    fn validate_server_ca_file_is_not_a_file() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let ca_file = PathBuf::from(dir.path());
        let expected = format!(
            "server CA file is invalid: file does not exist: {}",
            ca_file.to_string_lossy()
        );
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            ca_file: Some(ca_file.clone()),
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors.iter().any(|e| e.message.contains(&expected)));
    }

    #[test]
    fn validate_dns_refresh_interval_valid() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            dns_refresh_interval_seconds: 60,
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn validate_dns_refresh_interval_too_high() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            dns_refresh_interval_seconds: 3601,
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report.errors[0]
                .message
                .contains("invalid dns_refresh_interval_seconds: 3601seconds")
        );
    }

    #[test]
    fn validate_server_valid_pid_and_ca_files() {
        // Arrange
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();

        let pid_dir = dir.path().join("pid");
        std::fs::create_dir(&pid_dir).unwrap();

        let ca_file = dir.path().join("ca.pem");
        std::fs::write(&ca_file, "dummy").unwrap();

        let server = ServerSpec {
            pid_file: Some(pid_dir.join("snakeway.pid")),
            ca_file: Some(ca_file),
            ..Default::default()
        };

        // Act
        server.validate(&server.origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }
}
