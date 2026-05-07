use crate::types::{HclOrigin, IdentityDeviceSpec};
use crate::validation::validate_trusted_proxies;
use crate::validation::validator::{validate_geoip_db_file, validate_ua_parser_regexes_file};
use confval::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(MAX_X_FORWARDED_FOR_LENGTH, usize, min: 1, max: 2048);
range_constraint!(MAX_USER_AGENT_LENGTH, usize, min: 1, max: 4096);

impl ValidateSpec<HclOrigin> for IdentityDeviceSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        validate_trusted_proxies(&self.trusted_proxies, report, origin);

        validate_range_field!(
            MAX_X_FORWARDED_FOR_LENGTH,
            self.max_x_forwarded_for_length,
            report,
            origin
        );

        if self.enable_user_agent {
            validate_range_field!(
                MAX_USER_AGENT_LENGTH,
                self.max_user_agent_length,
                report,
                origin
            );
        }

        if self.enable_geoip {
            if let Some(path) = self.geoip_city_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }

            if let Some(path) = self.geoip_isp_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }

            if let Some(path) = self.geoip_connection_type_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }
        }

        if let Some(path) = self.ua_parser_regexes.as_ref() {
            validate_ua_parser_regexes_file(path, report, origin);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::IdentityDeviceSpec;
    use confval::{ValidateSpec, ValidationReport};

    #[test]
    fn max_xff_length_below_range() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = IdentityDeviceSpec {
            enable: true,
            max_x_forwarded_for_length: 0,
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("max_x_forwarded_for_length"))
        );
    }

    #[test]
    fn max_xff_length_above_range() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = IdentityDeviceSpec {
            enable: true,
            max_x_forwarded_for_length: 2049,
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("max_x_forwarded_for_length"))
        );
    }

    #[test]
    fn valid_identity_device() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = IdentityDeviceSpec {
            enable: true,
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(!report.has_issues());
    }
}
