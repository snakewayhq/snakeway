use crate::types::{NetworkPolicyDeviceSpec, OriginDeprecated};
use crate::validation::validator::validate_device_paths;
use crate::validation::{ValidateSpec, ValidationReportDeprecated};
use ipnet::IpNet;

impl ValidateSpec for NetworkPolicyDeviceSpec {
    fn validate(&self, origin: &OriginDeprecated, report: &mut ValidationReportDeprecated) {
        for cidr in &self.cidr_allow {
            if cidr.parse::<IpNet>().is_err() {
                report.invalid_network_policy_cidr(cidr, origin);
            }
        }

        validate_device_paths(&self.paths, report, origin);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::NetworkPolicyDeviceSpec;
    use crate::validation::{ValidateSpec, ValidationReportDeprecated};

    #[test]
    fn invalid_cidr_in_allow_list() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = NetworkPolicyDeviceSpec {
            enable: true,
            cidr_allow: vec!["not-a-cidr".to_string()],
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("invalid network policy CIDR"))
        );
    }

    #[test]
    fn valid_network_policy() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = NetworkPolicyDeviceSpec {
            enable: true,
            cidr_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn path_without_leading_slash_is_invalid() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = NetworkPolicyDeviceSpec {
            enable: true,
            cidr_allow: vec!["10.0.0.0/8".to_string()],
            paths: vec!["api/v1".to_string()],
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("must start with '/'"))
        );
    }
}
