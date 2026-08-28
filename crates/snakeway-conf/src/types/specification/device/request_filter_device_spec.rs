use crate::validation::validator::{HttpHeaderName, HttpMethod};
use confval::prelude::{AbsolutePath, Located, Report, Validate};
use confval::range_constraint;
use serde::Serialize;

range_constraint!(DENY_STATUS, i64, min: 400, max: 599);

#[derive(Clone, Debug, Serialize, confval::Spec)]
pub struct RequestFilterDeviceSpec {
    /// Whether this request filter device is enabled.
    pub enable: Located<bool>,

    #[confval(default, format = HttpMethod)]
    pub allow_methods: Vec<Located<String>>,

    #[confval(default, format = HttpMethod)]
    pub deny_methods: Vec<Located<String>>,

    #[confval(default, format = HttpHeaderName)]
    pub deny_headers: Vec<Located<String>>,

    #[confval(default, format = HttpHeaderName)]
    pub allow_headers: Vec<Located<String>>,

    #[confval(default, format = HttpHeaderName)]
    pub required_headers: Vec<Located<String>>,

    #[confval(default = 16 * 1024)]
    pub max_header_bytes: Located<i64>,
    #[confval(default = 1024 * 1024)]
    pub max_body_bytes: Located<i64>,
    #[confval(default = 8 * 1024)]
    pub max_suspicious_body_bytes: Located<i64>,

    /// Maximum time (in seconds) to wait for each chunk of request body data
    /// from the client.  If the client stalls mid-body for longer than this
    /// duration, the connection is terminated.  This prevents slowloris-style
    /// attacks that hold upstream resources by trickling body bytes.
    ///
    /// Applied to the downstream read timeout via Pingora's session API.
    /// `None` keeps Pingora's default (60 s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_body_timeout_seconds: Option<Located<i64>>,

    /// Override the default granular deny status with a device-scoped value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(range = DENY_STATUS)]
    pub deny_status: Option<Located<i64>>,

    /// Optional path prefixes this device applies to. Empty means all paths.
    #[confval(default, format = AbsolutePath)]
    pub paths: Vec<Located<String>>,
}

impl Default for RequestFilterDeviceSpec {
    fn default() -> Self {
        Self {
            enable: Located::detached(false),
            allow_methods: vec![],
            deny_methods: vec![],
            deny_headers: vec![],
            allow_headers: vec![],
            required_headers: vec![],
            max_header_bytes: Located::detached(16 * 1024),
            max_body_bytes: Located::detached(1024 * 1024),
            max_suspicious_body_bytes: Located::detached(8 * 1024),
            client_body_timeout_seconds: None,
            deny_status: None,
            paths: vec![],
        }
    }
}

impl Validate for RequestFilterDeviceSpec {
    fn validate(&self, _report: &mut Report) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spec_validates_clean() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestFilterDeviceSpec::default();

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn deny_status_below_range() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestFilterDeviceSpec {
            enable: Located::detached(true),
            deny_status: Some(Located::detached(399)),
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
                .any(|e| e.message.contains("deny_status"))
        );
    }

    #[test]
    fn deny_status_above_range() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestFilterDeviceSpec {
            enable: Located::detached(true),
            deny_status: Some(Located::detached(600)),
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
                .any(|e| e.message.contains("deny_status"))
        );
    }

    #[test]
    fn invalid_method_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestFilterDeviceSpec {
            enable: Located::detached(true),
            allow_methods: vec![Located::detached("G E T".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("invalid HTTP method"))
        );
    }

    #[test]
    fn invalid_header_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestFilterDeviceSpec {
            enable: Located::detached(true),
            deny_headers: vec![Located::detached("bad header".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("invalid HTTP header name"))
        );
    }

    #[test]
    fn valid_request_filter_device() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestFilterDeviceSpec {
            enable: Located::detached(true),
            allow_methods: vec![Located::detached("GET".to_string())],
            paths: vec![Located::detached("/api".to_string())],
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn disabled_device_is_still_validated() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestFilterDeviceSpec {
            enable: Located::detached(false),
            deny_status: Some(Located::detached(399)),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report.has_issues(),
            "a disabled device must still validate deny_status"
        );
    }
}
