use crate::validation::validator::validate_device_paths;
use confval::provenance::{Located, Report, Validate};
use ipnet::IpNet;
use serde::Serialize;

pub const ON_INVALID_FORWARDED: [&str; 2] = ["deny", "ignore"];

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

impl Validate for NetworkPolicyDeviceSpec {
    fn validate(&self, report: &mut Report) {
        for cidr in &self.cidr_allow {
            if cidr.value.parse::<IpNet>().is_err() {
                report
                    .error(format!("invalid network policy CIDR: {}", cidr.value))
                    .at(cidr.span)
                    .emit();
            }
        }

        if !ON_INVALID_FORWARDED.contains(&self.forwarding.value.on_invalid.value.as_str()) {
            report
                .error(format!(
                    "unknown on_invalid: {}",
                    self.forwarding.value.on_invalid.value
                ))
                .at(self.forwarding.value.on_invalid.span)
                .help(format!(
                    "expected one of: {}",
                    ON_INVALID_FORWARDED.join(", ")
                ))
                .emit();
        }

        validate_device_paths(&self.paths, report);
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
        spec.validate(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("invalid network policy CIDR"))
        );
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
        spec.validate(&mut report);

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
        spec.validate(&mut report);

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
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown on_invalid: explode")
        );
    }
}
