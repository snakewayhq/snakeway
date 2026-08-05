use crate::types::{IdentityField, LogEvent, LogLevel, LogPhase};
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;

#[derive(Clone, Debug, Serialize, confval::Spec)]
pub struct StructuredLoggingDeviceSpec {
    pub enable: Located<bool>,

    pub level: Located<String>,

    /// Headers are excluded by default.
    pub include_headers: Located<bool>,

    /// Allowlist of headers to include.
    /// If empty, all headers are eligible (subject to redaction).
    pub allowed_headers: Vec<Located<String>>,

    /// Headers to redact (case-insensitive).
    pub redacted_headers: Vec<Located<String>>,

    /// Identity logging.
    pub include_identity: Located<bool>,

    /// Identity fields to include in the request context (and possibly log).
    pub identity_fields: Vec<Located<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Located<Vec<Located<String>>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<Located<Vec<Located<String>>>>,
}

impl Default for StructuredLoggingDeviceSpec {
    fn default() -> Self {
        Self {
            enable: Located::detached(false),
            level: Located::detached("error".to_string()),
            include_headers: Located::detached(false),
            allowed_headers: vec![],
            redacted_headers: vec![],
            include_identity: Located::detached(false),
            identity_fields: vec![],
            events: None,
            phases: None,
        }
    }
}

impl Validate for StructuredLoggingDeviceSpec {
    fn validate(&self, report: &mut Report) {
        LogLevel::keyword_set().check_located(&self.level, "level", report);

        for field in &self.identity_fields {
            IdentityField::keyword_set().check_located(field, "identity field", report);
        }

        if let Some(events) = &self.events {
            for event in &events.value {
                LogEvent::keyword_set().check_located(event, "event", report);
            }
        }

        if let Some(phases) = &self.phases {
            for phase in &phases.value {
                LogPhase::keyword_set().check_located(phase, "phase", report);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_structured_logging_device() {
        // Arrange
        let mut report = Report::new();
        let spec = StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            level: Located::detached("info".to_string()),
            identity_fields: vec![Located::detached("client_ip".to_string())],
            events: Some(Located::detached(vec![Located::detached(
                "before_proxy".to_string(),
            )])),
            phases: Some(Located::detached(vec![Located::detached(
                "request".to_string(),
            )])),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn unknown_level_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            level: Located::detached("loud".to_string()),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown level: loud")
        );
    }

    #[test]
    fn unknown_identity_field_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            identity_fields: vec![Located::detached("shoe_size".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown identity field: shoe_size")
        );
    }

    #[test]
    fn unknown_event_and_phase_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            events: Some(Located::detached(vec![Located::detached(
                "during_proxy".to_string(),
            )])),
            phases: Some(Located::detached(vec![Located::detached(
                "midnight".to_string(),
            )])),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown event: during_proxy")
        );
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown phase: midnight")
        );
    }

    #[test]
    fn disabled_device_is_still_validated() {
        // Arrange
        let mut report = Report::new();
        let spec = StructuredLoggingDeviceSpec {
            enable: Located::detached(false),
            level: Located::detached("loud".to_string()),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report.has_issues(),
            "a disabled device must still validate its keyword values"
        );
    }
}
