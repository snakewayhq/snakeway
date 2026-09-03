use confval::prelude::{AbsolutePath, Located, Report, Validate, range_constraint};
use serde::Serialize;
use std::path::PathBuf;

const SIXTY_FOUR_GIB: i64 = 64 * 1024 * 1024 * 1024;
const SIXTY_FOUR_MIB: i64 = 64 * 1024 * 1024;
const ONE_YEAR_IN_SECONDS: i64 = 31_536_000;

// 1 byte to 64 GiB. A cap of zero would forbid every non-empty file.
range_constraint!(MAX_FILE_SIZE, i64, min: 1, max: SIXTY_FOUR_GIB, units: "bytes");
// 0 to 64 MiB. At zero, files are streamed rather than buffered in memory.
range_constraint!(SMALL_FILE_THRESHOLD, i64, min: 0, max: SIXTY_FOUR_MIB, units: "bytes");
// 0 to 64 MiB. At zero, every compressible response is gzipped.
range_constraint!(MIN_GZIP_SIZE, i64, min: 0, max: SIXTY_FOUR_MIB, units: "bytes");
// 0 to 64 MiB. At zero, every compressible response gets brotli.
range_constraint!(MIN_BROTLI_SIZE, i64, min: 0, max: SIXTY_FOUR_MIB, units: "bytes");
// 0 seconds to one year. Zero tells clients not to reuse the response.
range_constraint!(MAX_AGE_SECONDS, i64, min: 0, max: ONE_YEAR_IN_SECONDS, units: "seconds");

#[derive(Debug, Serialize, confval::Spec)]
pub struct StaticRouteSpec {
    pub hosts: Vec<Located<String>>,
    pub path: Located<String>,
    #[confval(format = AbsolutePath)]
    pub file_dir: Located<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<Located<String>>,
    pub directory_listing: Located<bool>,
    #[confval(range = MAX_FILE_SIZE)]
    pub max_file_size: Located<i64>,
    #[confval(nested)]
    pub compression: Located<CompressionOptsSpec>,
    #[confval(nested)]
    pub cache_policy: Located<CachePolicySpec>,
}

/// Every field is required in HCL, so these values apply only to specs built in Rust.
impl Default for StaticRouteSpec {
    fn default() -> Self {
        Self {
            hosts: Vec::new(),
            path: Located::detached(String::new()),
            file_dir: Located::detached(PathBuf::new()),
            index: None,
            directory_listing: Located::detached(false),
            max_file_size: Located::detached(10 * 1024 * 1024),
            compression: Located::detached(CompressionOptsSpec::default()),
            cache_policy: Located::detached(CachePolicySpec::default()),
        }
    }
}

impl Validate for StaticRouteSpec {
    fn validate(&self, report: &mut Report) {
        if !self.file_dir.value.exists() {
            report
                .error(format!(
                    "invalid static directory: {}",
                    self.file_dir.value.display()
                ))
                .at(self.file_dir.span)
                .emit();
        }
    }
}

#[derive(Debug, Clone, Serialize, confval::Spec)]
#[confval(derive_default)]
pub struct CompressionOptsSpec {
    #[confval(default = 256 * 1024, range = SMALL_FILE_THRESHOLD)]
    pub small_file_threshold: Located<i64>,
    #[confval(default = 1024, range = MIN_GZIP_SIZE)]
    pub min_gzip_size: Located<i64>,
    #[confval(default = 4 * 1024, range = MIN_BROTLI_SIZE)]
    pub min_brotli_size: Located<i64>,
    #[confval(default = true)]
    pub enable_gzip: Located<bool>,
    #[confval(default = true)]
    pub enable_brotli: Located<bool>,
}

impl Validate for CompressionOptsSpec {
    fn validate(&self, _report: &mut Report) {}
}

#[derive(Debug, Clone, Serialize, confval::Spec)]
#[confval(derive_default)]
pub struct CachePolicySpec {
    #[confval(default = 3600, range = MAX_AGE_SECONDS)]
    pub max_age_seconds: Located<i64>,
    #[confval(default = true)]
    pub public: Located<bool>,
    #[confval(default)]
    pub immutable: Located<bool>,
}

impl Validate for CachePolicySpec {
    fn validate(&self, _report: &mut Report) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_file_dir_does_not_exist() {
        // Arrange
        let file_dir = "/non/existent/static";
        let expected_error = format!("invalid static directory: {}", file_dir);
        let mut report = Report::new();
        let spec = StaticRouteSpec {
            file_dir: Located::detached(PathBuf::from(file_dir)),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert_eq!(report.issues().first().unwrap().message, expected_error);
    }

    #[test]
    fn static_file_dir_is_not_relative() {
        // Arrange
        let file_dir = "./www";
        let expected_error0 = format!("invalid static directory: {}", file_dir);
        let expected_error1 = format!("file_dir is not a valid absolute path: \"{}\"", file_dir);
        let mut report = Report::new();
        let spec = StaticRouteSpec {
            file_dir: Located::detached(PathBuf::from(file_dir)),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues()[0].message, expected_error1);
        assert_eq!(report.issues()[1].message, expected_error0);
    }

    #[test]
    fn max_file_size_of_zero_is_rejected() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let mut report = Report::new();
        let spec = StaticRouteSpec {
            file_dir: Located::detached(dir.path().to_path_buf()),
            max_file_size: Located::detached(0),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(
            report.issues()[0].message,
            "max_file_size must be at least 1"
        );
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("Set max_file_size to at least 1 bytes")
        );
    }

    #[test]
    fn max_file_size_above_maximum_is_rejected() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let mut report = Report::new();
        let spec = StaticRouteSpec {
            file_dir: Located::detached(dir.path().to_path_buf()),
            max_file_size: Located::detached(64 * 1024 * 1024 * 1024 + 1),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(
            report.issues()[0].message,
            "max_file_size must be at most 68719476736"
        );
    }

    #[test]
    fn compression_defaults_are_valid() {
        // Arrange
        let mut report = Report::new();
        let spec = CompressionOptsSpec::default();

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn min_gzip_size_above_maximum_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = CompressionOptsSpec {
            min_gzip_size: Located::detached(64 * 1024 * 1024 + 1),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(
            report.issues()[0].message,
            "min_gzip_size must be at most 67108864"
        );
    }

    #[test]
    fn disabled_encoder_thresholds_are_still_checked() {
        // Arrange
        let mut report = Report::new();
        let spec = CompressionOptsSpec {
            enable_gzip: Located::detached(false),
            enable_brotli: Located::detached(false),
            min_gzip_size: Located::detached(-1),
            min_brotli_size: Located::detached(-1),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("min_gzip_size")),
            "disabled gzip must still validate its threshold; issues: {:?}",
            report.issues()
        );
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("min_brotli_size")),
            "disabled brotli must still validate its threshold; issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn cache_max_age_below_minimum_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = CachePolicySpec {
            max_age_seconds: Located::detached(-1),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(
            report.issues()[0].message,
            "max_age_seconds must be at least 0"
        );
    }
}
