use crate::types::{HclOrigin, Http2Spec};
use confval::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(MAX_CONCURRENT_STREAMS, i64, min: 1, max: 4_294_967_295);
range_constraint!(MAX_HEADER_LIST_SIZE, i64, min: 1, max: 4_294_967_295, units: "bytes");
// RFC 9113 caps flow-control windows at 2^31 - 1; h2 asserts this during the
// connection handshake, so values above the cap must be rejected at config time.
range_constraint!(INITIAL_WINDOW_SIZE, i64, min: 1, max: 2_147_483_647, units: "bytes");
range_constraint!(INITIAL_CONNECTION_WINDOW_SIZE, i64, min: 1, max: 2_147_483_647, units: "bytes");

impl ValidateSpec<HclOrigin> for Http2Spec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        if let Some(max_concurrent_streams) = self.max_concurrent_streams {
            validate_range_field!(
                MAX_CONCURRENT_STREAMS,
                max_concurrent_streams,
                report,
                origin
            );
        }
        if let Some(max_header_list_size) = self.max_header_list_size {
            validate_range_field!(MAX_HEADER_LIST_SIZE, max_header_list_size, report, origin);
        }
        if let Some(initial_window_size) = self.initial_window_size {
            validate_range_field!(INITIAL_WINDOW_SIZE, initial_window_size, report, origin);
        }
        if let Some(initial_connection_window_size) = self.initial_connection_window_size {
            validate_range_field!(
                INITIAL_CONNECTION_WINDOW_SIZE,
                initial_connection_window_size,
                report,
                origin
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{HclOrigin, Http2Spec};
    use confval::{ValidateSpec, ValidationReport};

    fn test_origin() -> HclOrigin {
        HclOrigin::test("http2")
    }

    #[test]
    fn all_fields_unset_is_valid() {
        // Arrange
        let spec = Http2Spec::default();
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.errors().is_empty());
    }

    #[test]
    fn fields_at_range_boundaries_are_valid() {
        // Arrange
        let spec = Http2Spec {
            max_concurrent_streams: Some(1),
            max_header_list_size: Some(4_294_967_295),
            initial_window_size: Some(2_147_483_647),
            initial_connection_window_size: Some(2_147_483_647),
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.errors().is_empty());
    }

    #[test]
    fn zero_max_concurrent_streams_is_invalid() {
        // Arrange
        let spec = Http2Spec {
            max_concurrent_streams: Some(0),
            ..Default::default()
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
        assert!(
            report.errors()[0]
                .message
                .contains("max_concurrent_streams")
        );
    }

    #[test]
    fn initial_window_size_above_flow_control_cap_is_invalid() {
        // Arrange
        let spec = Http2Spec {
            initial_window_size: Some(2_147_483_648),
            ..Default::default()
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
        assert!(report.errors()[0].message.contains("initial_window_size"));
    }

    #[test]
    fn initial_connection_window_size_above_flow_control_cap_is_invalid() {
        // Arrange
        let spec = Http2Spec {
            initial_connection_window_size: Some(2_147_483_648),
            ..Default::default()
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
        assert!(
            report.errors()[0]
                .message
                .contains("initial_connection_window_size")
        );
    }

    #[test]
    fn negative_max_header_list_size_is_invalid() {
        // Arrange
        let spec = Http2Spec {
            max_header_list_size: Some(-1),
            ..Default::default()
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
        assert!(report.errors()[0].message.contains("max_header_list_size"));
    }
}
