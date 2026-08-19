use confval::prelude::{Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::{Deserialize, Serialize};

range_constraint!(MAX_CONCURRENT_STREAMS, i64, min: 1, max: 65535);
range_constraint!(MAX_HEADER_LIST_SIZE, i64, min: 1, max: 1_048_576, units: "bytes");
// RFC 9113 section 6.9.2: flow-control window cannot exceed 2^31 - 1.
range_constraint!(INITIAL_WINDOW_SIZE, i64, min: 1, max: 2_147_483_647, units: "bytes");
range_constraint!(INITIAL_CONNECTION_WINDOW_SIZE, i64, min: 1, max: 2_147_483_647, units: "bytes");

#[derive(Debug, Deserialize, Default, Serialize, Clone, confval::Spec)]
pub struct Http2Spec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent_streams: Option<Located<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_header_list_size: Option<Located<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_window_size: Option<Located<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_connection_window_size: Option<Located<i64>>,
}

impl Validate for Http2Spec {
    fn validate(&self, report: &mut Report) {
        if let Some(v) = &self.max_concurrent_streams {
            MAX_CONCURRENT_STREAMS.check_located(v, "max_concurrent_streams", report);
        }
        if let Some(v) = &self.max_header_list_size {
            MAX_HEADER_LIST_SIZE.check_located(v, "max_header_list_size", report);
        }
        if let Some(v) = &self.initial_window_size {
            INITIAL_WINDOW_SIZE.check_located(v, "initial_window_size", report);
        }
        if let Some(v) = &self.initial_connection_window_size {
            INITIAL_CONNECTION_WINDOW_SIZE.check_located(
                v,
                "initial_connection_window_size",
                report,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_defaults_pass() {
        // Arrange
        let mut report = Report::new();
        let spec = Http2Spec::default();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn zero_max_concurrent_streams_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = Http2Spec {
            max_concurrent_streams: Some(Located::detached(0)),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(report.has_errors());
        assert!(
            report.issues()[0]
                .message
                .contains("max_concurrent_streams must be at least 1")
        );
    }

    #[test]
    fn window_size_exceeding_rfc_limit_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = Http2Spec {
            initial_window_size: Some(Located::detached(2_147_483_648)),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(report.has_errors());
    }
}
