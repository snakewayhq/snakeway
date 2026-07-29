use crate::types::OnInvalidForwardedConfig;
use crate::validation::validator::{parse_cidr_list, validate_device_paths};
use confval::prelude::{ControlFlow, Located, Report, Validate};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, confval::Spec)]
pub struct NetworkPolicyDeviceSpec {
    pub enable: Located<bool>,
    pub cidr_allow: Vec<Located<String>>,
    #[confval(nested)]
    pub forwarding: Located<ForwardingSpec>,

    /// Optional path prefixes this device applies to. Empty means all paths.
    #[confval(default)]
    pub paths: Vec<Located<String>>,
}

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct ForwardingSpec {
    pub allow: Located<bool>,
    #[confval(default = "ignore".to_string())]
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
    fn validate(&self, report: &mut Report) {
        OnInvalidForwardedConfig::keyword_set().check_located(
            &self.on_invalid,
            "on_invalid",
            report,
        );
    }
}

impl Validate for NetworkPolicyDeviceSpec {
    fn validate(&self, report: &mut Report) {
        if !self.enable.value {
            return;
        }

        let _ = parse_cidr_list(&self.cidr_allow, "network policy allow list", report);

        validate_device_paths(&self.paths, report);
    }

    /// A disabled device describes nothing that runs, so its `forwarding`
    /// block is not worth reporting on.
    fn descend(&self) -> ControlFlow<()> {
        if self.enable.value {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                .any(|e| e.message.contains("must start with '/'"))
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
    fn disabled_device_skips_validation() {
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
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }
}
