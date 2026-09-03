use crate::types::OnInvalidForwarded;
use crate::validation::validator::parse_cidr_list;
use confval::prelude::{AbsolutePath, Located, Report, Validate};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, confval::Spec)]
pub struct NetworkPolicyDeviceSpec {
    pub enable: Located<bool>,
    pub cidr_allow: Vec<Located<String>>,
    #[confval(nested)]
    pub forwarding: Located<ForwardingSpec>,

    /// Optional path prefixes this device applies to. Empty means all paths.
    #[confval(default, format = AbsolutePath)]
    pub paths: Vec<Located<String>>,
}

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct ForwardingSpec {
    pub allow: Located<bool>,
    #[confval(default = "ignore".to_string(), keywords = OnInvalidForwarded)]
    pub on_invalid: Located<String>,
}

impl Default for ForwardingSpec {
    fn default() -> Self {
        Self {
            allow: Located::detached(false),
            on_invalid: Located::detached("ignore".to_string()),
        }
    }
}

impl Validate for ForwardingSpec {
    fn validate(&self, _report: &mut Report) {}
}

impl Validate for NetworkPolicyDeviceSpec {
    fn validate(&self, report: &mut Report) {
        let _ = parse_cidr_list(&self.cidr_allow, "network policy allow list", report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spec_validates_clean() {
        // Arrange
        let mut report = Report::new();
        let spec = NetworkPolicyDeviceSpec::default();

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn invalid_cidr_in_allow_list() {
        // Arrange
        let mut report = Report::new();
        let spec = NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: vec![Located::detached("not-a-cidr".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(report.issues().iter().any(|e| {
            e.message
                .contains("invalid CIDR in network policy allow list")
        }));
    }

    #[test]
    fn valid_network_policy() {
        // Arrange
        let mut report = Report::new();
        let spec = NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: vec![Located::detached("10.0.0.0/8".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn path_without_leading_slash_is_invalid() {
        // Arrange
        let mut report = Report::new();
        let spec = NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: vec![Located::detached("10.0.0.0/8".to_string())],
            paths: vec![Located::detached("api/v1".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "invalid absolute path in paths: \"api/v1\"")
        );
    }

    #[test]
    fn unknown_on_invalid_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: vec![Located::detached("10.0.0.0/8".to_string())],
            forwarding: Located::detached(ForwardingSpec {
                allow: Located::detached(true),
                on_invalid: Located::detached("explode".to_string()),
            }),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown on_invalid: explode")
        );
    }

    #[test]
    fn disabled_device_is_still_validated() {
        // Arrange
        let mut report = Report::new();
        let spec = NetworkPolicyDeviceSpec {
            enable: Located::detached(false),
            cidr_allow: vec![Located::detached("not-a-cidr".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report.has_issues(),
            "a disabled device must still validate its CIDR lists"
        );
    }
}
