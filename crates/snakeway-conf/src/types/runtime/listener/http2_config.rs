use crate::types::Http2Spec;
use confval::prelude::narrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = Http2Spec)]
pub struct Http2Config {
    #[confval(lower(from = max_concurrent_streams, with = narrow::opt_i64_to_u32))]
    pub max_concurrent_streams: Option<u32>,
    #[confval(lower(from = max_header_list_size, with = narrow::opt_i64_to_u32))]
    pub max_header_list_size: Option<u32>,
    #[confval(lower(from = initial_window_size, with = narrow::opt_i64_to_u32))]
    pub initial_window_size: Option<u32>,
    #[confval(lower(from = initial_connection_window_size, with = narrow::opt_i64_to_u32))]
    pub initial_connection_window_size: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Http2Spec;
    use confval::prelude::{Located, Lower, Report};

    #[test]
    fn lower_maps_all_fields() {
        // Arrange
        let spec = Http2Spec {
            max_concurrent_streams: Some(Located::detached(200)),
            max_header_list_size: Some(Located::detached(32768)),
            initial_window_size: Some(Located::detached(1_048_576)),
            initial_connection_window_size: Some(Located::detached(2_097_152)),
        };

        // Act
        let config = Http2Config::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert_eq!(config.max_concurrent_streams, Some(200));
        assert_eq!(config.max_header_list_size, Some(32768));
        assert_eq!(config.initial_window_size, Some(1_048_576));
        assert_eq!(config.initial_connection_window_size, Some(2_097_152));
    }

    #[test]
    fn lower_unset_fields_are_none() {
        // Arrange
        let spec = Http2Spec::default();

        // Act
        let config = Http2Config::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert_eq!(config.max_concurrent_streams, None);
        assert_eq!(config.max_header_list_size, None);
        assert_eq!(config.initial_window_size, None);
        assert_eq!(config.initial_connection_window_size, None);
    }

    #[test]
    fn negative_value_is_rejected() {
        // Arrange
        let spec = Http2Spec {
            max_concurrent_streams: Some(Located::detached(-1)),
            ..Default::default()
        };
        let mut report = Report::new();

        // Act
        let result = Http2Config::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(report.has_errors());
    }
}
