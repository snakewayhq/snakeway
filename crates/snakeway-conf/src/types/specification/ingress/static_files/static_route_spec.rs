use crate::types::HclInt;
use confval::provenance::{Located, Report};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct StaticRouteSpec {
    pub hosts: Vec<Located<String>>,
    pub path: Located<String>,
    pub file_dir: Located<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<Located<String>>,
    pub directory_listing: Located<bool>,
    pub max_file_size: Located<HclInt>,
    #[confval(nested)]
    pub compression: Located<CompressionOptsSpec>,
    #[confval(nested)]
    pub cache_policy: Located<CachePolicySpec>,
}

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct CompressionOptsSpec {
    #[confval(default = 256 * 1024)]
    pub small_file_threshold: Located<HclInt>,
    #[confval(default = 1024)]
    pub min_gzip_size: Located<HclInt>,
    #[confval(default = 4 * 1024)]
    pub min_brotli_size: Located<HclInt>,
    #[confval(default = true)]
    pub enable_gzip: Located<bool>,
    #[confval(default = true)]
    pub enable_brotli: Located<bool>,
}

impl Default for CompressionOptsSpec {
    fn default() -> Self {
        Self {
            small_file_threshold: Located::detached(256 * 1024), // 256 KiB
            min_gzip_size: Located::detached(1024),              // 1 KiB
            min_brotli_size: Located::detached(4 * 1024),        // 4 KiB
            enable_gzip: Located::detached(true),
            enable_brotli: Located::detached(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct CachePolicySpec {
    #[confval(default = 3600)]
    pub max_age_seconds: Located<HclInt>,
    #[confval(default = true)]
    pub public: Located<bool>,
    #[confval(default)]
    pub immutable: Located<bool>,
}

impl Default for CachePolicySpec {
    fn default() -> Self {
        Self {
            max_age_seconds: Located::detached(3600),
            public: Located::detached(true),
            immutable: Located::detached(false),
        }
    }
}

pub(crate) fn validate_static_route(spec: &StaticRouteSpec, report: &mut Report) {
    if !spec.file_dir.value.exists() {
        report
            .error(format!(
                "invalid static directory: {}",
                spec.file_dir.value.display()
            ))
            .at(spec.file_dir.span)
            .emit();
    }
    if spec.file_dir.value.is_relative() {
        report
            .error(format!(
                "static file directory must be an absolute path: {}",
                spec.file_dir.value.display()
            ))
            .at(spec.file_dir.span)
            .emit();
    }
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
        validate_static_route(&spec, &mut report);

        // Assert
        assert_eq!(report.issues().first().unwrap().message, expected_error);
    }

    #[test]
    fn static_file_dir_is_not_relative() {
        // Arrange
        let file_dir = "./www";
        let expected_error0 = format!("invalid static directory: {}", file_dir);
        let expected_error1 = format!(
            "static file directory must be an absolute path: {}",
            file_dir
        );
        let mut report = Report::new();
        let spec = StaticRouteSpec {
            file_dir: Located::detached(PathBuf::from(file_dir)),
            ..Default::default()
        };

        // Act
        validate_static_route(&spec, &mut report);

        // Assert
        assert_eq!(report.issues()[0].message, expected_error0);
        assert_eq!(report.issues()[1].message, expected_error1);
    }
}
